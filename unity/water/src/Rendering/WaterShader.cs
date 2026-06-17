using UnityEngine;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Represents a water-specific shader used by <see cref="WaterMaterial"/>.
    /// </summary>
    /// <remarks>
    /// This is a thin wrapper around Unity's <see cref="Shader"/> class that
    /// encapsulates the shader lookup by name.
    /// </remarks>
    /// <example>
    /// <code>
    /// var waterShader = new WaterShader("Phenotype/Water");
    /// var material = new WaterMaterial(waterShader);
    /// </code>
    /// </example>
    public class WaterShader
    {
        /// <summary>
        /// The underlying Unity shader instance.
        /// </summary>
        /// <value>
        /// The shader retrieved by <see cref="UnityEngine.Shader.Find(string)"/>.
        /// May be <c>null</c> if the shader name is not found.
        /// </value>
        /// <example>
        /// <code>
        /// var waterShader = new WaterShader("Phenotype/Water");
        /// if (waterShader.Shader == null)
        ///     Debug.LogError("Shader not found!");
        /// </code>
        /// </example>
        public Shader Shader { get; }

        /// <summary>
        /// Creates a water shader by loading the named shader via <see cref="UnityEngine.Shader.Find"/>.
        /// </summary>
        /// <param name="name">Shader name as registered in Unity.</param>
        /// <exception cref="System.ArgumentNullException">Thrown if <paramref name="name"/> is null.</exception>
        /// <remarks>
        /// The shader must be included in the build or available in the Resources folder.
        /// If the shader is not found, <see cref="Shader"/> will be <c>null</c>.
        /// </remarks>
        /// <example>
        /// <code>
        /// var shader = new WaterShader("Phenotype/Water");
        /// var material = new WaterMaterial(shader);
        /// </code>
        /// </example>
        public WaterShader(string name)
        {
            if (name == null)
                throw new System.ArgumentNullException(nameof(name));
            Shader = UnityEngine.Shader.Find(name);
        }
    }
}
