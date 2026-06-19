// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! IShaderAvailabilityProvider — port for shader-variant detection.
//! Adapters query this to validate that all required shader variants are loaded
//! before the pass runs (otherwise the pass would no-op or assert at runtime).

namespace Phenotype.PostFx.Ports
{
    /// <summary>
    /// Hexagonal port: asks the platform "is this shader available right now?"
    /// Adapters include DefaultShaderAvailabilityProvider (built-in),
    /// AddressablesShaderProvider, and AssetBundleShaderProvider.
    /// </summary>
    public interface IShaderAvailabilityProvider
    {
        /// <summary>
        /// Determines whether a shader with the specified name and keyword is available.
        /// </summary>
        /// <param name="shaderName">The name of the shader to check.</param>
        /// <param name="keyword">The shader keyword variant to validate.</param>
        /// <returns>
        /// <see langword="true"/> if the shader is loaded and the keyword is supported;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        bool IsAvailable(string shaderName, string keyword);
    }

    /// <summary>
    /// Default implementation that always returns <see langword="true"/>.
    /// Suitable for built-in render pipelines where all shader variants are guaranteed.
    /// </summary>
    public sealed class DefaultShaderAvailabilityProvider : IShaderAvailabilityProvider
    {
        /// <summary>
        /// Always returns <see langword="true"/>.
        /// </summary>
        /// <param name="shaderName">The name of the shader (ignored).</param>
        /// <param name="keyword">The shader keyword (ignored).</param>
        /// <returns><see langword="true"/>.</returns>
        public bool IsAvailable(string shaderName, string keyword) => true;
    }
}
