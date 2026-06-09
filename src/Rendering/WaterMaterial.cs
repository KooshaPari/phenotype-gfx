using UnityEngine;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Wraps a Unity <see cref="Material"/> configured for water rendering.
    /// </summary>
    public class WaterMaterial
    {
        private readonly Material _material;

        /// <summary>
        /// Creates a water material from the given shader.
        /// </summary>
        public WaterMaterial(WaterShader waterShader)
        {
            _material = new Material(waterShader.Shader);
        }

        /// <summary>
        /// The underlying Unity material instance.
        /// </summary>
        public Material Material => _material;

        /// <summary>Sets a float property on the material.</summary>
        public void SetFloat(string name, float value) => _material.SetFloat(name, value);

        /// <summary>Sets a Vector4 property on the material.</summary>
        public void SetVector(string name, Vector4 value) => _material.SetVector(name, value);

        /// <summary>Sets a texture property on the material.</summary>
        public void SetTexture(string name, Texture tex) => _material.SetTexture(name, tex);
    }
}
