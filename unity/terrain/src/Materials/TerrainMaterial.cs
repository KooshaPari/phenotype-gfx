using System;
using System.Collections.Generic;
using UnityEngine;

namespace Phenotype.Terrain.Materials
{
    /// <summary>
    /// Represents a material configuration for terrain rendering.
    /// Holds a collection of properties (textures, colors, scalars) that
    /// describe the surface appearance of a terrain chunk.
    /// </summary>
    /// <remarks>
    /// <para>Each material has a unique <see cref="Id"/> and a human-readable <see cref="Name"/>.
    /// Properties are stored in a dictionary keyed by name (case-insensitive).</para>
    /// <para>Use <see cref="AddProperty(TerrainMaterialProperty)"/> to populate the material,
    /// then query with <see cref="TryGetProperty(string, out TerrainMaterialProperty)"/> or
    /// <see cref="GetProperty(string)"/>.</para>
    /// </remarks>
    /// <example>
    /// <code>
    /// var material = new TerrainMaterial("Grass");
    /// material.BaseColor = Color.green;
    /// material.Smoothness = 0.2f;
    ///
    /// material.AddProperty(new TerrainMaterialProperty("Albedo", "Textures/Grass"));
    /// material.AddProperty(new TerrainMaterialProperty("Normal", "Textures/Grass_N"));
    ///
    /// if (material.TryGetProperty("Albedo", out var prop))
    ///     Debug.Log($"Texture: {prop.TexturePath}");
    /// </code>
    /// </example>
    public class TerrainMaterial
    {
        /// <summary>
        /// Unique identifier for this material.
        /// </summary>
        /// <value>Generated automatically as a new <see cref="Guid"/> at construction time.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Rock");
        /// Guid id = material.Id; // e.g., a1b2c3d4-...
        /// </code>
        /// </example>
        public Guid Id { get; }

        /// <summary>
        /// Human-readable name of the material.
        /// </summary>
        /// <value>Can be changed after construction. Must not be null or whitespace at construction.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Sand");
        /// material.Name = "Desert Sand";
        /// </code>
        /// </example>
        public string Name { get; set; }

        /// <summary>
        /// Base color tint applied to the terrain surface.
        /// </summary>
        /// <value>Defaults to <see cref="Color.white"/>. Multiplied with the albedo texture if present.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Mud");
        /// material.BaseColor = new Color(0.4f, 0.25f, 0.1f);
        /// </code>
        /// </example>
        public Color BaseColor { get; set; } = Color.white;

        /// <summary>
        /// Path to the primary diffuse texture (albedo map).
        /// </summary>
        /// <value>Defaults to <see cref="string.Empty"/>. Set to a project-relative texture path.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Forest");
        /// material.MainTexturePath = "Textures/Terrain/Forest_Diffuse";
        /// </code>
        /// </example>
        public string MainTexturePath { get; set; } = string.Empty;

        /// <summary>
        /// Path to the normal map texture.
        /// </summary>
        /// <value>Defaults to <see cref="string.Empty"/>. Set to a project-relative texture path.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Forest");
        /// material.NormalMapPath = "Textures/Terrain/Forest_Normal";
        /// </code>
        /// </example>
        public string NormalMapPath { get; set; } = string.Empty;

        /// <summary>
        /// Scale of the UV tiling for the main texture.
        /// </summary>
        /// <value>Defaults to 1. Higher values repeat the texture more frequently across the chunk.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Gravel");
        /// material.TextureScale = 8f; // 8x8 repeats per chunk
        /// </code>
        /// </example>
        public float TextureScale { get; set; } = 1f;

        /// <summary>
        /// Smoothness factor for the terrain surface.
        /// </summary>
        /// <value>Defaults to 0.5. Range 0 (rough) to 1 (mirror-like).</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Ice");
        /// material.Smoothness = 0.95f;
        /// </code>
        /// </example>
        public float Smoothness { get; set; } = 0.5f;

        /// <summary>
        /// Metallic factor for the terrain surface.
        /// </summary>
        /// <value>Defaults to 0 (dielectric). Range 0 to 1.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Ore");
        /// material.Metallic = 0.8f;
        /// </code>
        /// </example>
        public float Metallic { get; set; } = 0f;

        private readonly Dictionary<string, TerrainMaterialProperty> _properties;

        /// <summary>
        /// Creates a new terrain material with the specified name.
        /// </summary>
        /// <param name="name">Human-readable material name. Must not be null or whitespace.</param>
        /// <exception cref="ArgumentException">Thrown when <paramref name="name"/> is null or whitespace.</exception>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Volcanic Rock");
        /// material.BaseColor = Color.black;
        /// material.Metallic = 0.3f;
        /// </code>
        /// </example>
        public TerrainMaterial(string name)
        {
            if (string.IsNullOrWhiteSpace(name))
                throw new ArgumentException("Material name must not be null or empty.", nameof(name));

            Id = Guid.NewGuid();
            Name = name;
            _properties = new Dictionary<string, TerrainMaterialProperty>(StringComparer.OrdinalIgnoreCase);
        }

        /// <summary>
        /// Adds a property to the material.
        /// </summary>
        /// <param name="property">The property to add. Must not be null.</param>
        /// <exception cref="ArgumentNullException">Thrown when property is null.</exception>
        /// <exception cref="ArgumentException">Thrown when a property with the same name already exists.</exception>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Soil");
        /// material.AddProperty(new TerrainMaterialProperty("Moisture", 0.6f));
        /// material.AddProperty(new TerrainMaterialProperty("Albedo", "Textures/Soil"));
        /// </code>
        /// </example>
        public void AddProperty(TerrainMaterialProperty property)
        {
            if (property == null)
                throw new ArgumentNullException(nameof(property));
            if (_properties.ContainsKey(property.Name))
                throw new ArgumentException($"Property '{property.Name}' already exists.", nameof(property));

            _properties[property.Name] = property;
        }

        /// <summary>
        /// Removes a property from the material by name.
        /// </summary>
        /// <param name="name">Name of the property to remove. Case-insensitive.</param>
        /// <returns>True if the property was removed; false if it did not exist.</returns>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Debug");
        /// material.AddProperty(new TerrainMaterialProperty("Grid", 1f));
        /// bool removed = material.RemoveProperty("grid"); // true (case-insensitive)
        /// </code>
        /// </example>
        public bool RemoveProperty(string name)
        {
            return _properties.Remove(name);
        }

        /// <summary>
        /// Retrieves a property by name.
        /// </summary>
        /// <param name="name">Name of the property to retrieve. Case-insensitive.</param>
        /// <returns>The matching <see cref="TerrainMaterialProperty"/>.</returns>
        /// <exception cref="KeyNotFoundException">Thrown when the property does not exist.</exception>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Debug");
        /// material.AddProperty(new TerrainMaterialProperty("Grid", 1f));
        /// var prop = material.GetProperty("Grid");
        /// float val = prop.FloatValue; // 1f
        /// </code>
        /// </example>
        public TerrainMaterialProperty GetProperty(string name)
        {
            if (!_properties.TryGetValue(name, out var property))
                throw new KeyNotFoundException($"Property '{name}' not found on material '{Name}'.");
            return property;
        }

        /// <summary>
        /// Attempts to retrieve a property by name.
        /// </summary>
        /// <param name="name">Name of the property to retrieve. Case-insensitive.</param>
        /// <param name="property">When this method returns, contains the property if found; otherwise null.</param>
        /// <returns>True if the property was found; otherwise false.</returns>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Debug");
        /// if (material.TryGetProperty("Albedo", out var prop))
        ///     Debug.Log($"Found: {prop.Name}");
        /// else
        ///     Debug.Log("Property not found.");
        /// </code>
        /// </example>
        public bool TryGetProperty(string name, out TerrainMaterialProperty property)
        {
            return _properties.TryGetValue(name, out property);
        }

        /// <summary>
        /// Returns whether a property with the given name exists.
        /// </summary>
        /// <param name="name">Property name to check. Case-insensitive.</param>
        /// <returns>True if the property exists; otherwise false.</returns>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Debug");
        /// material.AddProperty(new TerrainMaterialProperty("AO", 0.5f));
        /// bool hasAo = material.HasProperty("ao"); // true (case-insensitive)
        /// </code>
        /// </example>
        public bool HasProperty(string name)
        {
            return _properties.ContainsKey(name);
        }

        /// <summary>
        /// Returns a read-only view of all property names.
        /// </summary>
        /// <value>A collection of property names. Empty if no properties have been added.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Debug");
        /// material.AddProperty(new TerrainMaterialProperty("A", 1f));
        /// material.AddProperty(new TerrainMaterialProperty("B", 2f));
        /// foreach (string name in material.PropertyNames)
        ///     Debug.Log(name);
        /// </code>
        /// </example>
        public IReadOnlyCollection<string> PropertyNames => _properties.Keys;

        /// <summary>
        /// Returns the total number of properties on this material.
        /// </summary>
        /// <value>The count of added properties. Zero for a newly created material.</value>
        /// <example>
        /// <code>
        /// var material = new TerrainMaterial("Debug");
        /// int count = material.PropertyCount; // 0
        /// material.AddProperty(new TerrainMaterialProperty("X", 1f));
        /// count = material.PropertyCount; // 1
        /// </code>
        /// </example>
        public int PropertyCount => _properties.Count;
    }
}
