using System;
using UnityEngine;

namespace Phenotype.Terrain.Materials
{
    /// <summary>
    /// Represents a single property of a terrain material, such as a texture,
    /// color, or scalar parameter used by the rendering pipeline.
    /// </summary>
    /// <remarks>
    /// Properties are strongly typed at construction. Once created, the <see cref="Type"/> cannot change.
    /// Use the appropriate typed accessor (<see cref="FloatValue"/>, <see cref="ColorValue"/>, etc.)
    /// to read or write the value.
    /// </remarks>
    /// <example>
    /// <code>
    /// var smoothness = new TerrainMaterialProperty("Smoothness", 0.8f);
    /// smoothness.FloatValue = 0.9f;
    ///
    /// var albedo = new TerrainMaterialProperty("Albedo", Color.green);
    /// albedo.ColorValue = Color.blue;
    /// </code>
    /// </example>
    public class TerrainMaterialProperty
    {
        /// <summary>
        /// Name of the property. Used as the key when binding to shaders.
        /// </summary>
        /// <value>Cannot be null or whitespace. Set at construction time.</value>
        /// <example>
        /// <code>
        /// var prop = new TerrainMaterialProperty("Metallic", 0.5f);
        /// string name = prop.Name; // "Metallic"
        /// </code>
        /// </example>
        public string Name { get; }

        /// <summary>
        /// Data type of this property.
        /// </summary>
        /// <value>Determined by the constructor overload used.</value>
        /// <example>
        /// <code>
        /// var prop = new TerrainMaterialProperty("Offset", Vector3.zero);
        /// TerrainMaterialPropertyType type = prop.Type; // TerrainMaterialPropertyType.Vector
        /// </code>
        /// </example>
        public TerrainMaterialPropertyType Type { get; }

        private float _floatValue;
        private Color _colorValue;
        private string _texturePath;
        private Vector3 _vectorValue;

        /// <summary>
        /// Creates a float property.
        /// </summary>
        /// <param name="name">Property name. Must not be null or whitespace.</param>
        /// <param name="value">Initial float value.</param>
        /// <exception cref="ArgumentException">Thrown when <paramref name="name"/> is null or whitespace.</exception>
        /// <example>
        /// <code>
        /// var roughness = new TerrainMaterialProperty("Roughness", 0.5f);
        /// </code>
        /// </example>
        public TerrainMaterialProperty(string name, float value)
        {
            if (string.IsNullOrWhiteSpace(name))
                throw new ArgumentException("Property name must not be null or empty.", nameof(name));

            Name = name;
            Type = TerrainMaterialPropertyType.Float;
            _floatValue = value;
        }

        /// <summary>
        /// Creates a color property.
        /// </summary>
        /// <param name="name">Property name. Must not be null or whitespace.</param>
        /// <param name="value">Initial color value.</param>
        /// <exception cref="ArgumentException">Thrown when <paramref name="name"/> is null or whitespace.</exception>
        /// <example>
        /// <code>
        /// var tint = new TerrainMaterialProperty("Tint", Color.cyan);
        /// </code>
        /// </example>
        public TerrainMaterialProperty(string name, Color value)
        {
            if (string.IsNullOrWhiteSpace(name))
                throw new ArgumentException("Property name must not be null or empty.", nameof(name));

            Name = name;
            Type = TerrainMaterialPropertyType.Color;
            _colorValue = value;
        }

        /// <summary>
        /// Creates a texture property.
        /// </summary>
        /// <param name="name">Property name. Must not be null or whitespace.</param>
        /// <param name="texturePath">Path or identifier for the texture asset.</param>
        /// <exception cref="ArgumentException">Thrown when <paramref name="name"/> is null or whitespace.</exception>
        /// <example>
        /// <code>
        /// var diffuse = new TerrainMaterialProperty("Diffuse", "Textures/Grass_Diffuse");
        /// </code>
        /// </example>
        public TerrainMaterialProperty(string name, string texturePath)
        {
            if (string.IsNullOrWhiteSpace(name))
                throw new ArgumentException("Property name must not be null or empty.", nameof(name));

            Name = name;
            Type = TerrainMaterialPropertyType.Texture;
            _texturePath = texturePath ?? string.Empty;
        }

        /// <summary>
        /// Creates a vector property.
        /// </summary>
        /// <param name="name">Property name. Must not be null or whitespace.</param>
        /// <param name="value">Initial vector value.</param>
        /// <exception cref="ArgumentException">Thrown when <paramref name="name"/> is null or whitespace.</exception>
        /// <example>
        /// <code>
        /// var tiling = new TerrainMaterialProperty("Tiling", new Vector3(4f, 4f, 0f));
        /// </code>
        /// </example>
        public TerrainMaterialProperty(string name, Vector3 value)
        {
            if (string.IsNullOrWhiteSpace(name))
                throw new ArgumentException("Property name must not be null or empty.", nameof(name));

            Name = name;
            Type = TerrainMaterialPropertyType.Vector;
            _vectorValue = value;
        }

        /// <summary>
        /// Gets or sets the float value. Only valid when <see cref="Type"/> is <see cref="TerrainMaterialPropertyType.Float"/>.
        /// </summary>
        /// <value>A scalar float.</value>
        /// <exception cref="InvalidOperationException">Thrown when the property is not a float type.</exception>
        /// <example>
        /// <code>
        /// var prop = new TerrainMaterialProperty("Shininess", 0.5f);
        /// prop.FloatValue = 0.8f;
        /// float current = prop.FloatValue; // 0.8f
        /// </code>
        /// </example>
        public float FloatValue
        {
            get
            {
                if (Type != TerrainMaterialPropertyType.Float)
                    throw new InvalidOperationException($"Property '{Name}' is not a float type.");
                return _floatValue;
            }
            set
            {
                if (Type != TerrainMaterialPropertyType.Float)
                    throw new InvalidOperationException($"Property '{Name}' is not a float type.");
                _floatValue = value;
            }
        }

        /// <summary>
        /// Gets or sets the color value. Only valid when <see cref="Type"/> is <see cref="TerrainMaterialPropertyType.Color"/>.
        /// </summary>
        /// <value>A <see cref="Color"/> value.</value>
        /// <exception cref="InvalidOperationException">Thrown when the property is not a color type.</exception>
        /// <example>
        /// <code>
        /// var prop = new TerrainMaterialProperty("Highlight", Color.yellow);
        /// prop.ColorValue = Color.red;
        /// Color current = prop.ColorValue; // Color.red
        /// </code>
        /// </example>
        public Color ColorValue
        {
            get
            {
                if (Type != TerrainMaterialPropertyType.Color)
                    throw new InvalidOperationException($"Property '{Name}' is not a color type.");
                return _colorValue;
            }
            set
            {
                if (Type != TerrainMaterialPropertyType.Color)
                    throw new InvalidOperationException($"Property '{Name}' is not a color type.");
                _colorValue = value;
            }
        }

        /// <summary>
        /// Gets or sets the texture path. Only valid when <see cref="Type"/> is <see cref="TerrainMaterialPropertyType.Texture"/>.
        /// </summary>
        /// <value>A path string. Never null; returns <see cref="string.Empty"/> if not set.</value>
        /// <exception cref="InvalidOperationException">Thrown when the property is not a texture type.</exception>
        /// <example>
        /// <code>
        /// var prop = new TerrainMaterialProperty("NormalMap", "Textures/Normal");
        /// prop.TexturePath = "Textures/Normal_HighRes";
        /// string path = prop.TexturePath; // "Textures/Normal_HighRes"
        /// </code>
        /// </example>
        public string TexturePath
        {
            get
            {
                if (Type != TerrainMaterialPropertyType.Texture)
                    throw new InvalidOperationException($"Property '{Name}' is not a texture type.");
                return _texturePath;
            }
            set
            {
                if (Type != TerrainMaterialPropertyType.Texture)
                    throw new InvalidOperationException($"Property '{Name}' is not a texture type.");
                _texturePath = value ?? string.Empty;
            }
        }

        /// <summary>
        /// Gets or sets the vector value. Only valid when <see cref="Type"/> is <see cref="TerrainMaterialPropertyType.Vector"/>.
        /// </summary>
        /// <value>A <see cref="Vector3"/> value.</value>
        /// <exception cref="InvalidOperationException">Thrown when the property is not a vector type.</exception>
        /// <example>
        /// <code>
        /// var prop = new TerrainMaterialProperty("WindDirection", Vector3.up);
        /// prop.VectorValue = new Vector3(1f, 0f, 0f);
        /// Vector3 dir = prop.VectorValue; // (1, 0, 0)
        /// </code>
        /// </example>
        public Vector3 VectorValue
        {
            get
            {
                if (Type != TerrainMaterialPropertyType.Vector)
                    throw new InvalidOperationException($"Property '{Name}' is not a vector type.");
                return _vectorValue;
            }
            set
            {
                if (Type != TerrainMaterialPropertyType.Vector)
                    throw new InvalidOperationException($"Property '{Name}' is not a vector type.");
                _vectorValue = value;
            }
        }
    }
}
