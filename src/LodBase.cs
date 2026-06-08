using System;

namespace Phenotype.Terrain
{
    /// <summary>
    /// Resolution tier returned by LOD selection.
    /// </summary>
    public enum LodTier
    {
        /// <summary>Camera is within near distance — highest grid density.</summary>
        Near,
        /// <summary>Camera is within mid distance.</summary>
        Mid,
        /// <summary>Camera is within far distance — coarsest rendered grid.</summary>
        Far,
        /// <summary>Camera is beyond the cull distance — mesh should not be rendered.</summary>
        Culled,
    }

    /// <summary>
    /// Base class for level-of-detail selection based on camera distance.
    /// Subclasses override the distance thresholds and per-tier resolution.
    /// </summary>
    public abstract class LodBase
    {
        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Near"/> to <see cref="LodTier.Mid"/> quality.
        /// </summary>
        public abstract float NearDistance { get; }

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Mid"/> to <see cref="LodTier.Far"/> quality.
        /// </summary>
        public abstract float MidDistance { get; }

        /// <summary>
        /// Distance (in world units) at which the mesh transitions from
        /// <see cref="LodTier.Far"/> to <see cref="LodTier.Culled"/>.
        /// Geometry beyond this distance is not rendered.
        /// </summary>
        public abstract float CullDistance { get; }

        /// <summary>
        /// Returns the LOD tier appropriate for the given camera distance.
        /// </summary>
        /// <param name="distance">Camera-to-mesh distance in world units. Must be >= 0.</param>
        /// <returns>The <see cref="LodTier"/> for this distance.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when distance is negative.</exception>
        public LodTier SelectTier(float distance)
        {
            if (distance < 0f)
                throw new ArgumentOutOfRangeException(nameof(distance), "distance must be >= 0");

            if (distance < NearDistance)   return LodTier.Near;
            if (distance < MidDistance)    return LodTier.Mid;
            if (distance < CullDistance)   return LodTier.Far;
            return LodTier.Culled;
        }

        /// <summary>
        /// Validates that the configured thresholds are monotonically increasing:
        /// NearDistance &lt; MidDistance &lt; CullDistance.
        /// </summary>
        /// <exception cref="InvalidOperationException">Thrown if thresholds are not monotonically increasing.</exception>
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
