using System;

namespace Phenotype.Terrain
{
    /// <summary>
    /// Manages level-of-detail selection for terrain chunks based on camera distance.
    /// Coordinates mesh swap-ins to keep draw calls bounded at wide zoom levels.
    /// </summary>
    public class TerrainLod : LodBase
    {
        /// <summary>Distance to <see cref="LodTier.Mid"/> transition. Default: 50.</summary>
        public override float NearDistance { get; } = 50f;

        /// <summary>Distance to <see cref="LodTier.Far"/> transition. Default: 150.</summary>
        public override float MidDistance { get; } = 150f;

        /// <summary>Distance to <see cref="LodTier.Culled"/> transition. Default: 400.</summary>
        public override float CullDistance { get; } = 400f;

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
        public int SelectResolution(float distance)
        {
            return SelectTier(distance) switch
            {
                LodTier.Near   => NearResolution,
                LodTier.Mid    => MidResolution,
                LodTier.Far    => FarResolution,
                LodTier.Culled => 0,
                _              => 0,
            };
        }
    }
}
