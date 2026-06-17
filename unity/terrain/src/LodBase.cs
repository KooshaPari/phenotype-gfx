using System;

namespace Phenotype.Terrain
{
    /// <summary>
    /// Resolution tier returned by LOD selection.
    /// </summary>
    /// <remarks>
    /// Tiers are ordered from highest detail to lowest detail. <see cref="Culled"/>
    /// signals that the mesh should not be rendered at all.
    /// </remarks>
    /// <example>
    /// <code>
    /// LodTier tier = lod.SelectTier(75f);
    /// if (tier == LodTier.Culled)
    ///     meshRenderer.enabled = false;
    /// else
    ///     meshRenderer.enabled = true;
    /// </code>
    /// </example>
    public enum LodTier
    {
        /// <summary>Camera is within near distance — highest grid density.</summary>
        /// <remarks>Typically uses the highest resolution mesh (e.g., 64 quads per side).</remarks>
        Near,
        /// <summary>Camera is within mid distance.</summary>
        /// <remarks>Typically uses a medium resolution mesh (e.g., 32 quads per side).</remarks>
        Mid,
        /// <summary>Camera is within far distance — coarsest rendered grid.</summary>
        /// <remarks>Typically uses the lowest resolution mesh (e.g., 16 quads per side).</remarks>
        Far,
        /// <summary>Camera is beyond the cull distance — mesh should not be rendered.</summary>
        /// <remarks>When this tier is selected the chunk should be hidden entirely to save draw calls.</remarks>
        Culled,
    }

    /// <summary>
    /// Base class for level-of-detail selection based on camera distance.
    /// Subclasses override the distance thresholds and per-tier resolution.
    /// </summary>
    /// <remarks>
    /// Thresholds must be strictly monotonic: <c>NearDistance &lt; MidDistance &lt; CullDistance</c>.
    /// Call <see cref="ValidateThresholds"/> after modifying distances to ensure consistency.
    /// </remarks>
    /// <example>
    /// <code>
    /// public class CustomLod : LodBase
    /// {
    ///     public override float NearDistance { get; set; } = 25f;
    ///     public override float MidDistance { get; set; } = 100f;
    ///     public override float CullDistance { get; set; } = 300f;
    /// }
    ///
    /// var lod = new CustomLod();
    /// lod.ValidateThresholds();
    /// LodTier tier = lod.SelectTier(50f); // LodTier.Mid
    /// </code>
    /// </example>
    public abstract class LodBase
    {
        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Near"/> to <see cref="LodTier.Mid"/> quality.
        /// </summary>
        /// <value>Must be less than <see cref="MidDistance"/>.</value>
        /// <example>
        /// <code>
        /// lod.NearDistance = 50f;
        /// </code>
        /// </example>
        public abstract float NearDistance { get; set; }

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Mid"/> to <see cref="LodTier.Far"/> quality.
        /// </summary>
        /// <value>Must be greater than <see cref="NearDistance"/> and less than <see cref="CullDistance"/>.</value>
        /// <example>
        /// <code>
        /// lod.MidDistance = 150f;
        /// </code>
        /// </example>
        public abstract float MidDistance { get; set; }

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Far"/> to <see cref="LodTier.Culled"/>.
        /// Geometry beyond this distance is not rendered.
        /// </summary>
        /// <value>Must be greater than <see cref="MidDistance"/>.</value>
        /// <example>
        /// <code>
        /// lod.CullDistance = 400f;
        /// </code>
        /// </example>
        public abstract float CullDistance { get; set; }

        /// <summary>
        /// Returns the LOD tier appropriate for the given camera distance.
        /// </summary>
        /// <param name="distance">Camera-to-mesh distance in world units. Must be >= 0.</param>
        /// <returns>The <see cref="LodTier"/> for this distance.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when distance is negative.</exception>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// LodTier tierNear = lod.SelectTier(25f);   // LodTier.Near
        /// LodTier tierMid = lod.SelectTier(100f);   // LodTier.Mid
        /// LodTier tierFar = lod.SelectTier(300f);   // LodTier.Far
        /// LodTier tierCull = lod.SelectTier(500f);  // LodTier.Culled
        /// </code>
        /// </example>
        public LodTier SelectTier(float distance)
        {
            if (distance < 0f)
                throw new ArgumentOutOfRangeException(nameof(distance), "distance must be >= 0");

            if (distance < NearDistance) return LodTier.Near;
            if (distance < MidDistance) return LodTier.Mid;
            if (distance < CullDistance) return LodTier.Far;
            return LodTier.Culled;
        }

        /// <summary>
        /// Validates that the configured thresholds are monotonically increasing:
        /// NearDistance &lt; MidDistance &lt; CullDistance.
        /// </summary>
        /// <exception cref="InvalidOperationException">Thrown if thresholds are not monotonically increasing.</exception>
        /// <example>
        /// <code>
        /// var lod = new TerrainLod();
        /// lod.NearDistance = 100f;
        /// lod.MidDistance = 50f; // Invalid: Near > Mid
        /// lod.ValidateThresholds(); // Throws InvalidOperationException
        /// </code>
        /// </example>
        public void ValidateThresholds()
        {
            if (NearDistance >= MidDistance)
                throw new InvalidOperationException(
                    $"NearDistance ({NearDistance}) must be less than MidDistance ({MidDistance}).");
            if (MidDistance >= CullDistance)
                throw new InvalidOperationException(
                    $"MidDistance ({MidDistance}) must be less than CullDistance ({CullDistance}).");
        }
    }
}
