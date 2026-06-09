using UnityEngine;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Represents a water-specific shader used by <see cref="WaterMaterial"/>.
    /// </summary>
    public class WaterShader
    {
        /// <summary>
        /// The underlying Unity shader instance.
        /// </summary>
        public Shader Shader { get; }

        /// <summary>
        /// Creates a water shader by loading the named shader via <see cref="UnityEngine.Shader.Find"/>.
        /// </summary>
        /// <param name="name">Shader name as registered in Unity.</param>
        public WaterShader(string name)
        {
            Shader = UnityEngine.Shader.Find(name);
        }
    }
}
