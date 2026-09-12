// UnityStubs.cs — Minimal UnityEngine type stubs for headless test runs.
//
// The real unity/PhenotypeGfx.cs wrapper pulls in `UnityEngine.Vector3`,
// `UnityEngine.Vector2`, and `UnityEngine.Mesh`. To exercise the FFI
// lifecycle from a plain `dotnet test` runner (no Unity install, no
// UnityEditor) we provide just enough surface for those types to be
// usable from compiled test code.
//
// Behaviour is deliberately trivial: we are not testing Unity, we are
// testing that our C# wrapper correctly marshals data and respects
// handle ownership. The tests assert observable side effects (vertex
// counts, disposal flags, etc.), not Unity rendering semantics.

using System;

namespace UnityEngine
{
    /// <summary>
    /// Minimal stub of UnityEngine.Vector3. Only the fields used by the
    /// wrapper (x/y/z) are exposed; arithmetic helpers are omitted on
    /// purpose so the stub has no false sense of completeness.
    /// </summary>
    public struct Vector3 : IEquatable<Vector3>
    {
        public float x;
        public float y;
        public float z;

        public Vector3(float x, float y, float z)
        {
            this.x = x;
            this.y = y;
            this.z = z;
        }

        public bool Equals(Vector3 other) =>
            x.Equals(other.x) && y.Equals(other.y) && z.Equals(other.z);

        public override bool Equals(object obj) => obj is Vector3 v && Equals(v);

        public override int GetHashCode() => HashCode.Combine(x, y, z);

        public override string ToString() => $"({x}, {y}, {z})";
    }

    /// <summary>
    /// Minimal stub of UnityEngine.Vector2 (x, y).
    /// </summary>
    public struct Vector2 : IEquatable<Vector2>
    {
        public float x;
        public float y;

        public Vector2(float x, float y)
        {
            this.x = x;
            this.y = y;
        }

        public bool Equals(Vector2 other) => x.Equals(other.x) && y.Equals(other.y);

        public override bool Equals(object obj) => obj is Vector2 v && Equals(v);

        public override int GetHashCode() => HashCode.Combine(x, y);

        public override string ToString() => $"({x}, {y})";
    }

    /// <summary>
    /// Minimal stub of UnityEngine.Mesh that records writes to its
    /// vertex/normal/uv/triangle arrays. Tests can inspect the cached
    /// arrays after <c>FillUnityMesh</c> runs.
    /// </summary>
    public class Mesh
    {
        public string name { get; set; } = string.Empty;

        public Vector3[] vertices { get; set; } = Array.Empty<Vector3>();

        public Vector3[] normals { get; set; } = Array.Empty<Vector3>();

        public Vector2[] uv { get; set; } = Array.Empty<Vector2>();

        public int[] triangles { get; set; } = Array.Empty<int>();

        /// <summary>Number of times Clear() was called. Tests assert on this.</summary>
        public int ClearCallCount { get; private set; }

        public void Clear()
        {
            vertices = Array.Empty<Vector3>();
            normals = Array.Empty<Vector3>();
            uv = Array.Empty<Vector2>();
            triangles = Array.Empty<int>();
            ClearCallCount++;
        }
    }
}
