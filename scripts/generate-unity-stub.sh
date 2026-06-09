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

# Build the stub assembly
dotnet build "${STUB_DIR}/Stub.csproj" -c Release --nologo

# Emit the path that should be used as $(WorldBoxManaged)
STUB_DLL_DIR="${STUB_DIR}/bin/Release/net8.0"
if [[ -n "${GITHUB_ENV:-}" ]]; then
    echo "WorldBoxManaged=${STUB_DLL_DIR}" >> "${GITHUB_ENV}"
fi

echo "UnityEngine.CoreModule stub built at: ${STUB_DLL_DIR}"
