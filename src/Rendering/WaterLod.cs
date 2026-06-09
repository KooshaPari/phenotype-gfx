using System;
using Phenotype.Terrain;

namespace Phenotype.Water.Rendering
{
    /// <summary>
    /// Controls level-of-detail for the water surface mesh.
    /// Adjusts vertex density and wave evaluation frequency based on camera distance,
    /// ensuring consistent frame budget at all zoom levels.
    /// </summary>
    /// <remarks>
    /// Inherits distance thresholds from <see cref="LodBase"/> and adds per-tier
    /// grid resolutions specific to water rendering.
    /// </remarks>
    /// <example>
    /// <code>
    /// var lod = new WaterLod();
    /// int resolution = lod.SelectResolution(75f); // Returns MidResolution (32)
    /// </code>
    /// </example>
    public class WaterLod : LodBase
    {
        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Near"/> to <see cref="LodTier.Mid"/> quality.
        /// </summary>
        /// <value>Default: 50. Must be less than <see cref="MidDistance"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new WaterLod();
        /// lod.NearDistance = 40f;
        /// lod.ValidateThresholds();
        /// </code>
        /// </example>
        public override float NearDistance { get; set; } = 50f;

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Mid"/> to <see cref="LodTier.Far"/> quality.
        /// </summary>
        /// <value>Default: 150. Must be greater than <see cref="NearDistance"/> and less than <see cref="CullDistance"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new WaterLod();
        /// lod.MidDistance = 120f;
        /// lod.ValidateThresholds();
        /// </code>
        /// </example>
        public override float MidDistance { get; set; } = 150f;

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Far"/> to <see cref="LodTier.Culled"/>.
        /// Geometry beyond this distance is not rendered.
        /// </summary>
        /// <value>Default: 400. Must be greater than <see cref="MidDistance"/>.</value>
        /// <example>
        /// <code>
        /// var lod = new WaterLod();
        /// lod.CullDistance = 500f;
        /// lod.ValidateThresholds();
        /// </code>
        /// </example>
        public override float CullDistance { get; set; } = 400f;

        /// <summary>
        /// Grid resolution used for the <see cref="LodTier.Near"/> tier.
        /// </summary>
        /// <value>Default: 64. Higher values = more vertices = smoother waves.</value>
        /// <example>
        /// <code>
        /// var lod = new WaterLod();
        /// lod.NearResolution = 128; // High fidelity near the camera
        /// </code>
        /// </example>
        public int NearResolution { get; set; } = 64;

        /// <summary>
        /// Grid resolution used for the <see cref="LodTier.Mid"/> tier.
        /// </summary>
        /// <value>Default: 32. Balanced quality for mid-range viewing.</value>
        /// <example>
        /// <code>
        /// var lod = new WaterLod();
        /// lod.MidResolution = 24;
        /// </code>
        /// </example>
        public int MidResolution { get; set; } = 32;

        /// <summary>
        /// Grid resolution used for the <see cref="LodTier.Far"/> tier.
        /// </summary>
        /// <value>Default: 16. Low resolution for distant water.</value>
        /// <example>
        /// <code>
        /// var lod = new WaterLod();
        /// lod.FarResolution = 8; // Very low resolution for distant patches
        /// </code>
        /// </example>
        public int FarResolution { get; set; } = 16;

        /// <summary>
        /// Returns the grid resolution appropriate for the given camera distance.
        /// </summary>
        /// <param name="distance">Camera-to-water distance in world units. Must be >= 0.</param>
        /// <returns>
        /// One of <see cref="NearResolution"/>, <see cref="MidResolution"/>,
        /// <see cref="FarResolution"/>, or 0 if the mesh should be culled.
        /// </returns>
        /// <remarks>
        /// Delegates to <see cref="LodBase.SelectTier(float)"/> to determine the tier,
        /// then maps the tier to the corresponding resolution property.
        /// </remarks>
        /// <example>
        /// <code>
        /// var lod = new WaterLod();
        /// int nearRes = lod.SelectResolution(25f);   // 64
        /// int midRes = lod.SelectResolution(100f);   // 32
        /// int farRes = lod.SelectResolution(250f);   // 16
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
