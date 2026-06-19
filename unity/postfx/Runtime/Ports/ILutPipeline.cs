// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! T40 spec: HDR LUT pipeline (Gen-5 RNP) hexagonal port.
//!
//! Supports 4 LUT formats: .cube, .3dl, .csp, and PNG-encoded Hald images.
//! Each format is an adapter; the pipeline can be swapped at runtime via the
//! config (e.g., "use .cube in editor, .csp in shipping").

using UnityEngine;

namespace Phenotype.PostFx.Lut
{
    /// <summary>
    /// Supported LUT (Look-Up Table) file formats.
    /// </summary>
    public enum LutFormat { Cube, ThreeDl, Csp, HaldPng }

    /// <summary>
    /// Parsed LUT data including format, size, HDR flag, color samples, and source path.
    /// </summary>
    public sealed class LutData
    {
        /// <summary>
        /// Gets the file format of this LUT.
        /// </summary>
        /// <value>The LUT format.</value>
        public LutFormat Format { get; }

        /// <summary>
        /// Gets the grid size of the LUT.
        /// Typically 17, 33, or 65 for .cube files; 8-256 for others.
        /// </summary>
        /// <value>The grid dimension.</value>
        public int Size { get; }

        /// <summary>
        /// Gets a value indicating whether this LUT stores HDR values.
        /// </summary>
        /// <value>
        /// <see langword="true"/> if the LUT is HDR; otherwise, <see langword="false"/>.
        /// </value>
        public bool IsHdr { get; }

        /// <summary>
        /// Gets the flattened color array.
        /// Length is <c>Size^3</c> for 3D LUTs or <c>Size*Size</c> for 1D LUTs.
        /// </summary>
        /// <value>The color samples.</value>
        public Color[] Colors { get; }

        /// <summary>
        /// Gets the source path or identifier of this LUT.
        /// </summary>
        /// <value>The source path.</value>
        public string Source { get; }

        /// <summary>
        /// Initializes a new instance of the <see cref="LutData"/> class.
        /// </summary>
        /// <param name="format">The LUT file format.</param>
        /// <param name="size">The grid size.</param>
        /// <param name="isHdr">Whether the LUT stores HDR values.</param>
        /// <param name="colors">The flattened color array.</param>
        /// <param name="source">The source path or identifier.</param>
        public LutData(LutFormat format, int size, bool isHdr, Color[] colors, string source)
        {
            Format = format; Size = size; IsHdr = isHdr; Colors = colors; Source = source;
        }

        /// <summary>
        /// Creates an identity LUT that performs no color transformation.
        /// </summary>
        /// <param name="format">The desired LUT format.</param>
        /// <param name="size">
        /// The grid size. Defaults to 17.
        /// </param>
        /// <returns>
        /// A <see cref="LutData"/> instance where each color maps to itself.
        /// </returns>
        /// <example>
        /// <code>
        /// var identity = LutData.Identity(LutFormat.Cube, 33);
        /// </code>
        /// </example>
        public static LutData Identity(LutFormat format, int size = 17)
        {
            var colors = new Color[size * size * size];
            for (int b = 0; b < size; b++)
            for (int g = 0; g < size; g++)
            for (int r = 0; r < size; r++)
                colors[(b * size + g) * size + r] = new Color((float)r / (size-1), (float)g / (size-1), (float)b / (size-1), 1f);
            return new LutData(format, size, false, colors, "<identity>");
        }
    }

    /// <summary>
    /// Hexagonal port: parses and serializes a LUT in a specific format.
    /// Each format (Cube, 3DL, CSP, Hald PNG) has its own adapter.
    /// </summary>
    public interface ILutAdapter
    {
        /// <summary>
        /// Gets the format this adapter handles.
        /// </summary>
        /// <value>The LUT format.</value>
        LutFormat Format { get; }

        /// <summary>
        /// Parses a LUT file from the specified path.
        /// </summary>
        /// <param name="path">Absolute or relative path to the LUT file.</param>
        /// <returns>The parsed <see cref="LutData"/>.</returns>
        LutData Parse(string path);

        /// <summary>
        /// Attempts to parse a LUT file from the specified path.
        /// </summary>
        /// <param name="path">Absolute or relative path to the LUT file.</param>
        /// <param name="data">
        /// When this method returns, contains the parsed <see cref="LutData"/> if successful,
        /// or <see langword="null"/> if parsing failed.
        /// </param>
        /// <returns>
        /// <see langword="true"/> if parsing succeeded; otherwise, <see langword="false"/>.
        /// </returns>
        bool TryParse(string path, out LutData data);

        /// <summary>
        /// Serializes LUT data to the specified path.
        /// </summary>
        /// <param name="path">The destination file path.</param>
        /// <param name="data">The LUT data to serialize.</param>
        void Serialize(string path, LutData data);
    }

    /// <summary>
    /// Hexagonal port: the LUT pipeline (parse → validate → upload to GPU).
    /// </summary>
    public interface ILutPipeline
    {
        /// <summary>
        /// Gets the currently loaded LUT data.
        /// </summary>
        /// <value>The active LUT, or <see langword="null"/> if none is loaded.</value>
        LutData Current { get; }

        /// <summary>
        /// Loads a LUT from the specified path.
        /// </summary>
        /// <param name="path">Absolute or relative path to the LUT file.</param>
        void Load(string path);

        /// <summary>
        /// Replaces the current LUT with an identity transform (no color change).
        /// </summary>
        void UseIdentity();

        /// <summary>
        /// Validates the supplied LUT data.
        /// </summary>
        /// <param name="data">The LUT data to validate.</param>
        /// <param name="error">
        /// When this method returns, contains a descriptive error message if validation failed,
        /// or <see langword="null"/> if validation succeeded.
        /// </param>
        /// <returns>
        /// <see langword="true"/> if the LUT is valid; otherwise, <see langword="false"/>.
        /// </returns>
        bool Validate(LutData data, out string error);

        /// <summary>
        /// Converts the LUT data into a GPU-ready <see cref="Texture2D"/>.
        /// </summary>
        /// <param name="data">The LUT data to upload.</param>
        /// <returns>A 2D texture representing the LUT.</returns>
        Texture2D ToTexture2D(LutData data);
    }

    /// <summary>
    /// Helper methods for analyzing LUT data.
    /// </summary>
    public static class LutPipelineHelpers
    {
        /// <summary>
        /// Determines whether the LUT data is an identity transform (no color change).
        /// </summary>
        /// <param name="data">The LUT data to test.</param>
        /// <param name="tolerance">
        /// The maximum deviation from identity to still consider the LUT an identity.
        /// Defaults to 0.001f.
        /// </param>
        /// <returns>
        /// <see langword="true"/> if the LUT is identity within the given tolerance;
        /// otherwise, <see langword="false"/>.
        /// </returns>
        public static bool IsIdentity(LutData data, float tolerance = 0.001f) { /* ... */ return false; }

        /// <summary>
        /// Determines whether the LUT data is in sRGB color space.
        /// </summary>
        /// <param name="data">The LUT data to test.</param>
        /// <returns>
        /// <see langword="true"/> if the LUT is sRGB; otherwise, <see langword="false"/>.
        /// </returns>
        public static bool IsSrgb(LutData data) { /* ... */ return true; }
    }
}
