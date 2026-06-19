using UnityEngine;

namespace Phenotype.PostFx
{
    /// <summary>
    /// Minimal post-processing pass interface. Every pass (built-in or custom)
    /// implements this so the <see cref="PostStack"/> driver can initialise,
    /// render, and clean up passes without knowing their concrete type.
    /// </summary>
    public interface IPostFxPass
    {
        /// <summary>
        /// Binds the owning <see cref="PostStack"/>.
        /// </summary>
        /// <param name="owner">The <see cref="PostStack"/> that owns this pass.</param>
        /// <remarks>Called once after construction.</remarks>
        void Init(PostStack owner);

        /// <summary>
        /// Renders the pass from <paramref name="src"/> into <paramref name="dst"/>.
        /// </summary>
        /// <param name="src">The source render texture.</param>
        /// <param name="dst">The destination render texture.</param>
        /// <returns>
        /// <see langword="true"/> if the pass wrote to <paramref name="dst"/>;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        bool Render(RenderTexture src, RenderTexture dst);

        /// <summary>
        /// Releases pass-specific resources.
        /// </summary>
        /// <remarks>Called when the stack is destroyed.</remarks>
        void Dispose();
    }
}
