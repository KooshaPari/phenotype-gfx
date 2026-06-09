using UnityEngine;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Wraps a Unity <see cref="Material"/> configured for water rendering.
    /// </summary>
    /// <remarks>
    /// Provides a strongly typed API for setting common water shader properties
    /// such as wave strength, foam colour, and normal maps.
    /// </remarks>
    /// <example>
    /// <code>
    /// var shader = new WaterShader("Phenotype/Water");
    /// var material = new WaterMaterial(shader);
    /// material.SetFloat("_WaveStrength", 0.5f);
    /// material.SetVector("_DeepColor", new Vector4(0, 0.1f, 0.2f, 1f));
    /// </code>
    /// </example>
    public class WaterMaterial
    {
        private readonly Material _material;

        /// <summary>
        /// Creates a water material from the given shader.
        /// </summary>
        /// <param name="waterShader">The water shader to use for this material.</param>
        /// <exception cref="System.ArgumentNullException">Thrown if <paramref name="waterShader"/> is null.</exception>
        /// <example>
        /// <code>
        /// var shader = new WaterShader("Phenotype/Water");
        /// var material = new WaterMaterial(shader);
        /// </code>
        /// </example>
        public WaterMaterial(WaterShader waterShader)
        {
            _material = new Material(waterShader.Shader);
        }

        /// <summary>
        /// The underlying Unity material instance.
        /// </summary>
        /// <value>The native <see cref="Material"/> created from the water shader.</value>
        /// <example>
        /// <code>
        /// var material = new WaterMaterial(shader);
        /// meshRenderer.material = material.Material;
        /// </code>
        /// </example>
        public Material Material => _material;

        /// <summary>
        /// Sets a float property on the material.
        /// </summary>
        /// <param name="name">Property name as defined in the shader.</param>
        /// <param name="value">Float value to assign.</param>
        /// <example>
        /// <code>
        /// material.SetFloat("_WaveStrength", 0.5f);
        /// </code>
        /// </example>
        public void SetFloat(string name, float value) => _material.SetFloat(name, value);

        /// <summary>
        /// Sets a Vector4 property on the material.
        /// </summary>
        /// <param name="name">Property name as defined in the shader.</param>
        /// <param name="value">Vector4 value to assign.</param>
        /// <example>
        /// <code>
        /// material.SetVector("_DeepColor", new Vector4(0, 0.1f, 0.2f, 1f));
        /// </code>
        /// </example>
        public void SetVector(string name, Vector4 value) => _material.SetVector(name, value);

        /// <summary>
        /// Sets a texture property on the material.
        /// </summary>
        /// <param name="name">Property name as defined in the shader.</param>
        /// <param name="tex">Texture to assign.</param>
        /// <example>
        /// <code>
        /// material.SetTexture("_NormalMap", normalTexture);
        /// </code>
        /// </example>
        public void SetTexture(string name, Texture tex) => _material.SetTexture(name, tex);
    }
}
