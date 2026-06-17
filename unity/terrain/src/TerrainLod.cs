using System;

namespace Phenotype.Terrain
{
    /// <summary>
    /// Manages level-of-detail selection for terrain chunks based on camera distance.
    /// Coordinates mesh swap-ins to keep draw calls bounded at wide zoom levels.
    /// </summary>
    /// <remarks>
    /// Default thresholds are <c>50 / 150 / 400</c> world units and default resolutions are
    /// <c>64 / 32 / 16</c> quads per side. Override these values to tune for your scene scale.
    /// </remarks>
    /// <example>
    /// <code>
    /// var lod = new TerrainLod
    /// {
    ///     NearDistance = 30f,
    ///     MidDistance = 120f,
    ///     CullDistance = 500f,
    ///     NearResolution = 128,
    ///     MidResolution = 64,
    ///     FarResolution = 32,
    /// };
    ///
    /// int res = lod.SelectResolution(80f); // 64
    /// </code>
    /// </example>
    public class TerrainLod : LodBase
    {
        /// <summary>
        /// Distance to <see cref="LodTier.Mid"/> transition. Default: 50.
        /// </summary>
        /// <value>Must be less than <see cref="MidDistance"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// lod.NearDistance = 40f;
        /// </code>
        /// </example>
        public override float NearDistance { get; set; } = 50f;

        /// <summary>
        /// Distance to <see cref="LodTier.Far"/> transition. Default: 150.
        /// </summary>
        /// <value>Must be greater than <see cref="NearDistance"/> and less than <see cref="CullDistance"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// lod.MidDistance = 200f;
        /// </code>
        /// </example>
        public override float MidDistance { get; set; } = 150f;

        /// <summary>
        /// Distance to <see cref="LodTier.Culled"/> transition. Default: 400.
        /// </summary>
        /// <value>Must be greater than <see cref="MidDistance"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// lod.CullDistance = 1000f;
        /// </code>
        /// </example>
        public override float CullDistance { get; set; } = 400f;

        /// <summary>
        /// Grid resolution used for the <see cref="LodTier.Near"/> tier. Default: 64.
        /// </summary>
        /// <value>Must be a positive integer.</value>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// lod.NearResolution = 128;
        /// </code>
        /// </example>
        public int NearResolution { get; set; } = 64;

        /// <summary>
        /// Grid resolution used for the <see cref="LodTier.Mid"/> tier. Default: 32.
        /// </summary>
        /// <value>Must be a positive integer. Typically half of <see cref="NearResolution"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// lod.MidResolution = 48;
        /// </code>
        /// </example>
        public int MidResolution { get; set; } = 32;

        /// <summary>
        /// Grid resolution used for the <see cref="LodTier.Far"/> tier. Default: 16.
        /// </summary>
        /// <value>Must be a positive integer. Typically half of <see cref="MidResolution"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// lod.FarResolution = 8;
        /// </code>
        /// </example>
        public int FarResolution { get; set; } = 16;

        /// <summary>
        /// Returns the grid resolution appropriate for the given camera distance.
        /// Returns 0 when the mesh should be culled.
        /// </summary>
        /// <param name="distance">Camera-to-mesh distance in world units. Must be >= 0.</param>
        /// <returns>
        /// The grid resolution for the selected <see cref="LodTier"/>:
        /// <list type="table">
        /// <listheader><term>Tier</term><description>Resolution</description></listheader>
        /// <item><term><see cref="LodTier.Near"/></term><description><see cref="NearResolution"/></description></item>
        /// <item><term><see cref="LodTier.Mid"/></term><description><see cref="MidResolution"/></description></item>
        /// <item><term><see cref="LodTier.Far"/></term><description><see cref="FarResolution"/></description></item>
        /// <item><term><see cref="LodTier.Culled"/></term><description>0</description></item>
        /// </list>
        /// </returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when distance is negative.</exception>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// int nearRes = lod.SelectResolution(25f);  // 64
        /// int midRes = lod.SelectResolution(100f);   // 32
        /// int farRes = lod.SelectResolution(300f);   // 16
        /// int culled = lod.SelectResolution(500f);   // 0
        /// </code>
        /// </example>
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
