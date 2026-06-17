using System;

namespace Phenotype.Terrain
{
    /// <summary>
    /// Stores and queries per-tile elevation data for terrain height sampling.
    /// Consumers call into this to resolve world-space Y for a given tile coordinate.
    /// </summary>
    /// <remarks>
    /// The height field stores data in a flat float array indexed as <c>z * Width + x</c>.
    /// Coordinates are zero-based and range from <c>(0, 0)</c> to <c>(Width - 1, Height - 1)</c>.
    /// </remarks>
    /// <example>
    /// <code>
    /// var hf = new HeightField(256, 256);
    /// hf.SetHeight(10, 10, 42.0f);
    /// float y = hf.GetHeight(10, 10); // 42.0f
    /// </code>
    /// </example>
    public class HeightField
    {
        private readonly float[] _data;

        /// <summary>
        /// Width of the height field in tiles.
        /// </summary>
        public int Width { get; }

        /// <summary>
        /// Height of the height field in tiles.
        /// </summary>
        public int Height { get; }

        /// <summary>
        /// Creates a new height field with the specified dimensions and optional elevation data.
        /// </summary>
        /// <param name="width">Number of tiles along the X axis. Must be non-negative.</param>
        /// <param name="height">Number of tiles along the Z axis. Must be non-negative.</param>
        /// <param name="data">Optional flat array of elevation values. When null, a zero-initialized array is used.</param>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when width or height is negative.</exception>
        /// <exception cref="ArgumentException">Thrown when data length does not match width * height.</exception>
        /// <example>
        /// <code>
        /// // Create a flat 64x64 height field
        /// var hf = new HeightField(64, 64);
        ///
        /// // Create from existing data
        /// float[] data = new float[256];
        /// var hf2 = new HeightField(16, 16, data);
        /// </code>
        /// </example>
        public HeightField(int width, int height, float[] data = null)
        {
            if (width < 0)
                throw new ArgumentOutOfRangeException(nameof(width), "Width must be non-negative.");
            if (height < 0)
                throw new ArgumentOutOfRangeException(nameof(height), "Height must be non-negative.");

            int expectedLength = width * height;
            if (data != null && data.Length != expectedLength)
                throw new ArgumentException(
                    $"Data length ({data.Length}) does not match expected size ({expectedLength}).",
                    nameof(data));

            Width = width;
            Height = height;
            _data = data ?? new float[expectedLength];
        }

        /// <summary>
        /// Returns the elevation at the given tile coordinate.
        /// </summary>
        /// <param name="x">Tile coordinate along the X axis. Must be within <c>[0, Width)</c>.</param>
        /// <param name="z">Tile coordinate along the Z axis. Must be within <c>[0, Height)</c>.</param>
        /// <returns>Elevation value in world units.</returns>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when coordinates are out of bounds.</exception>
        /// <example>
        /// <code>
        /// var hf = new HeightField(64, 64);
        /// hf.SetHeight(5, 5, 10.0f);
        /// float elevation = hf.GetHeight(5, 5); // 10.0f
        /// </code>
        /// </example>
        public float GetHeight(int x, int z)
        {
            if (x < 0 || x >= Width)
                throw new ArgumentOutOfRangeException(nameof(x), "X coordinate is out of bounds.");
            if (z < 0 || z >= Height)
                throw new ArgumentOutOfRangeException(nameof(z), "Z coordinate is out of bounds.");

            return _data[z * Width + x];
        }

        /// <summary>
        /// Sets the elevation at the given tile coordinate.
        /// </summary>
        /// <param name="x">Tile coordinate along the X axis. Must be within <c>[0, Width)</c>.</param>
        /// <param name="z">Tile coordinate along the Z axis. Must be within <c>[0, Height)</c>.</param>
        /// <param name="value">Elevation value in world units.</param>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when coordinates are out of bounds.</exception>
        /// <example>
        /// <code>
        /// var hf = new HeightField(64, 64);
        /// hf.SetHeight(0, 0, 100.0f);
        /// hf.SetHeight(63, 63, 50.0f);
        /// </code>
        /// </example>
        public void SetHeight(int x, int z, float value)
        {
            if (x < 0 || x >= Width)
                throw new ArgumentOutOfRangeException(nameof(x), "X coordinate is out of bounds.");
            if (z < 0 || z >= Height)
                throw new ArgumentOutOfRangeException(nameof(z), "Z coordinate is out of bounds.");

            _data[z * Width + x] = value;
        }

        /// <summary>
        /// Returns a copy of the internal elevation data array.
        /// </summary>
        /// <returns>A new array containing the current elevation values in row-major order (z * Width + x).</returns>
        /// <example>
        /// <code>
        /// var hf = new HeightField(4, 4);
        /// hf.SetHeight(1, 2, 5.0f);
        /// float[] snapshot = hf.GetData();
        /// // snapshot[2 * 4 + 1] == 5.0f
        /// </code>
        /// </example>
        public float[] GetData()
        {
            float[] copy = new float[_data.Length];
            _data.CopyTo(copy, 0);
            return copy;
        }
    }
}
