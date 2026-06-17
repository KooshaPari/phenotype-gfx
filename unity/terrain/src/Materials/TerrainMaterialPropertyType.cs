using UnityEngine;

namespace Phenotype.Terrain.Materials
{
    /// <summary>
    /// Defines the data type of a terrain material property.
    /// </summary>
    /// <remarks>
    /// Each type corresponds to a specific backing field in <see cref="TerrainMaterialProperty"/>.
    /// Accessing the wrong typed value (e.g., <see cref="TerrainMaterialProperty.FloatValue"/> on a
    /// <see cref="Texture"/> property) will throw <see cref="InvalidOperationException"/>.
    /// </remarks>
    /// <example>
    /// <code>
    /// var prop = new TerrainMaterialProperty("Albedo", Color.red);
    /// TerrainMaterialPropertyType type = prop.Type; // TerrainMaterialPropertyType.Color
    /// </code>
    /// </example>
    public enum TerrainMaterialPropertyType
    {
        /// <summary>A scalar float value.</summary>
        /// <remarks>Stored in <see cref="TerrainMaterialProperty.FloatValue"/>.</remarks>
        Float,
        /// <summary>A 4-component color value.</summary>
        /// <remarks>Stored in <see cref="TerrainMaterialProperty.ColorValue"/>.</remarks>
        Color,
        /// <summary>A 2D texture reference.</summary>
        /// <remarks>Stored as a path string in <see cref="TerrainMaterialProperty.TexturePath"/>.</remarks>
        Texture,
        /// <summary>A 3-component vector value.</summary>
        /// <remarks>Stored in <see cref="TerrainMaterialProperty.VectorValue"/>.</remarks>
        Vector,
    }
}
