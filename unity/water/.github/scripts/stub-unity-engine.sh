#!/usr/bin/env bash
# Usage: stub-unity-engine.sh [--source FILE] [--framework TFM] [--assembly NAME]
# Builds a stub UnityEngine.CoreModule.dll from the provided C# source file.
# In CI, appends WorldBoxManaged=<output-dir> to GITHUB_ENV.

set -euo pipefail

SOURCE_FILE=""
TARGET_FRAMEWORK="net48"
ASSEMBLY_NAME="UnityEngine.CoreModule"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source|-s)
      SOURCE_FILE="$2"
      shift 2
      ;;
    --framework|-f)
      TARGET_FRAMEWORK="$2"
      shift 2
      ;;
    --assembly|-a)
      ASSEMBLY_NAME="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# Default source file: prefer local src/UnityEngineStubs.cs
if [[ -z "$SOURCE_FILE" ]]; then
  if [[ -f "src/UnityEngineStubs.cs" ]]; then
    SOURCE_FILE="$(pwd)/src/UnityEngineStubs.cs"
  else
    echo "No source file specified and src/UnityEngineStubs.cs not found." >&2
    exit 1
  fi
fi

# Resolve absolute path
SOURCE_FILE="$(cd "$(dirname "$SOURCE_FILE")" && pwd)/$(basename "$SOURCE_FILE")"

STUB_DIR=$(mktemp -d)

cat > "$STUB_DIR/Stub.csproj" <<EOF
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>${TARGET_FRAMEWORK}</TargetFramework>
    <AssemblyName>${ASSEMBLY_NAME}</AssemblyName>
    <RootNamespace>UnityEngine</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <Compile Include="${SOURCE_FILE}" />
  </ItemGroup>
</Project>
EOF

dotnet build "$STUB_DIR/Stub.csproj" -c Release --nologo

OUTPUT_DIR="$STUB_DIR/bin/Release/${TARGET_FRAMEWORK}"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "WorldBoxManaged=${OUTPUT_DIR}" >> "$GITHUB_ENV"
fi

echo "Stub built: ${OUTPUT_DIR}/${ASSEMBLY_NAME}.dll"
