#!/usr/bin/env bash
# Generate a minimal UnityEngine.CoreModule stub assembly for CI builds.
# Works on both macOS and Linux.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Allow overriding the stub output directory via STUB_DIR env var or first arg
STUB_DIR="${1:-${STUB_DIR:-${PROJECT_ROOT}/_stub}}"
rm -rf "${STUB_DIR}"
mkdir -p "${STUB_DIR}"

# Write the stub project file
cat > "${STUB_DIR}/Stub.csproj" <<'EOF'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <AssemblyName>UnityEngine.CoreModule</AssemblyName>
    <RootNamespace>UnityEngine</RootNamespace>
  </PropertyGroup>
</Project>
EOF

# Write the stub C# source files
cat > "${STUB_DIR}/Vector3.cs" <<'EOF'
namespace UnityEngine
{
    public struct Vector3
    {
        public float x, y, z;
        public Vector3(float x, float y, float z) { this.x = x; this.y = y; this.z = z; }

        // Arithmetic operators used by tests (e.g. ChunkMeshBuilderTests computes edge vectors).
        public static Vector3 operator -(Vector3 a, Vector3 b) => new Vector3(a.x - b.x, a.y - b.y, a.z - b.z);
        public static Vector3 operator +(Vector3 a, Vector3 b) => new Vector3(a.x + b.x, a.y + b.y, a.z + b.z);
        public static Vector3 operator -(Vector3 a) => new Vector3(-a.x, -a.y, -a.z);
        public static Vector3 operator *(Vector3 a, float s) => new Vector3(a.x * s, a.y * s, a.z * s);
        public static Vector3 operator *(float s, Vector3 a) => new Vector3(a.x * s, a.y * s, a.z * s);

        // Static vector constants — used by Vector3.one, Vector3.zero, Vector3.up, etc.
        public static Vector3 zero => new Vector3(0f, 0f, 0f);
        public static Vector3 one => new Vector3(1f, 1f, 1f);
        public static Vector3 up => new Vector3(0f, 1f, 0f);
        public static Vector3 down => new Vector3(0f, -1f, 0f);
        public static Vector3 left => new Vector3(-1f, 0f, 0f);
        public static Vector3 right => new Vector3(1f, 0f, 0f);
        public static Vector3 forward => new Vector3(0f, 0f, 1f);
        public static Vector3 back => new Vector3(0f, 0f, -1f);
    }
}
EOF

cat > "${STUB_DIR}/Vector2.cs" <<'EOF'
namespace UnityEngine
{
    public struct Vector2
    {
        public float x, y;
        public Vector2(float x, float y) { this.x = x; this.y = y; }
    }
}
EOF

cat > "${STUB_DIR}/Color.cs" <<'EOF'
namespace UnityEngine
{
    public struct Color
    {
        public float r, g, b, a;
        public Color(float r, float g, float b, float a) { this.r = r; this.g = g; this.b = b; this.a = a; }
        public Color(float r, float g, float b) : this(r, g, b, 1f) { }

        // Static color constants — used by Color.white, Color.red, etc.
        public static Color white => new Color(1f, 1f, 1f, 1f);
        public static Color black => new Color(0f, 0f, 0f, 1f);
        public static Color red => new Color(1f, 0f, 0f, 1f);
        public static Color green => new Color(0f, 1f, 0f, 1f);
        public static Color blue => new Color(0f, 0f, 1f, 1f);
        public static Color yellow => new Color(1f, 1f, 0f, 1f);
        public static Color cyan => new Color(0f, 1f, 1f, 1f);
        public static Color magenta => new Color(1f, 0f, 1f, 1f);
        public static Color gray => new Color(0.5f, 0.5f, 0.5f, 1f);
        public static Color clear => new Color(0f, 0f, 0f, 0f);
    }
}
EOF

# Build the stub assembly
dotnet build "${STUB_DIR}/Stub.csproj" -c Release --nologo

# Emit the path that should be used as $(WorldBoxManaged)
STUB_DLL_DIR="${STUB_DIR}/bin/Release/net8.0"
if [[ -n "${GITHUB_ENV:-}" ]]; then
    echo "WorldBoxManaged=${STUB_DLL_DIR}" >> "${GITHUB_ENV}"
fi

echo "UnityEngine.CoreModule stub built at: ${STUB_DLL_DIR}"
