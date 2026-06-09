using System;
using Phenotype.Terrain;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Controls level-of-detail for the water surface mesh.
    /// Adjusts vertex density and wave evaluation frequency based on camera distance,
    /// ensuring consistent frame budget at all zoom levels.
    /// </summary>
    public class WaterLod : LodBase
    {
        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Near"/> to <see cref="LodTier.Mid"/> quality.
        /// Default: 50.
        /// </summary>
        public override float NearDistance { get; set; } = 50f;

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Mid"/> to <see cref="LodTier.Far"/> quality.
        /// Default: 150.
        /// </summary>
        public override float MidDistance { get; set; } = 150f;

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Far"/> to <see cref="LodTier.Culled"/>.
        /// Geometry beyond this distance is not rendered.
        /// Default: 400.
        /// </summary>
        public override float CullDistance { get; set; } = 400f;

        /// <summary>Grid resolution used for the <see cref="LodTier.Near"/> tier. Default: 64.</summary>
        public int NearResolution { get; set; } = 64;

        /// <summary>Grid resolution used for the <see cref="LodTier.Mid"/> tier. Default: 32.</summary>
        public int MidResolution { get; set; } = 32;

        /// <summary>Grid resolution used for the <see cref="LodTier.Far"/> tier. Default: 16.</summary>
        public int FarResolution { get; set; } = 16;

        /// <summary>
        /// Returns the grid resolution appropriate for the given camera distance.
        /// Returns 0 when the mesh should be culled.
        /// </summary>
        /// <param name="distance">Camera-to-water distance in world units. Must be >= 0.</param>
        /// <returns>
        /// One of <see cref="NearResolution"/>, <see cref="MidResolution"/>,
        /// <see cref="FarResolution"/>, or 0 if culled.
        /// </returns>
        public int SelectResolution(float distance)
        {
            return SelectTier(distance) switch
            {
                LodTier.Near => NearResolution,
                LodTier.Mid => MidResolution,
                LodTier.Far => FarResolution,
                LodTier.Culled => 0,
                _ => 0,
            };
        }
    }
}
