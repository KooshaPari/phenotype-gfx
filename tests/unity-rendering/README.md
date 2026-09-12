# Tests/Unity-rendering

Headless Unity Editor tests that exercise the **7 phenotype-postfx passes**
(SSAO, SSGI, Bloom, ACES, Vignette, Chromatic, LUT) through the **real Unity
rendering pipeline** (`Graphics.Blit` + `RenderTexture`).

Unlike the existing `unity/postfx/tests/Editor/PostStackEditTests.cs` which run
under `dotnet test` with a Unity-stub assembly, this project compiles against
the real UnityEngine and dispatches each pass as the BRP post-processing chain
does at runtime. The intent is to catch shader-compile failures, dead-code
optimisations that drop a pass, and regressions in the tonemap/colour-grading
math.

## Layout

```
tests/unity-rendering/
├── Assets/
│   ├── Editor/
│   │   ├── Phenotype.PostFx.EditorTests.asmdef
│   │   └── PostStackShaderRenderingTests.cs   # 7 NUnit tests, one per pass
│   └── Scenes/                                  # Procedurally built at test time
├── Packages/
│   └── manifest.json                            # Pins com.phenotype.postfx
└── ProjectSettings/
    └── ProjectVersion.txt                       # Unity 2022.3.20f1
```

## Run locally

```bash
# EditMode tests
Unity -batchmode -nographics -runTests \
      -testPlatform EditMode \
      -projectPath tests/unity-rendering \
      -testResults tests/unity-rendering/results.xml \
      -logFile tests/unity-rendering/unity-test.log
```

`-nographics` is required because `unityci/editor` does not ship an X server,
but `Graphics.Blit` still works against off-screen `RenderTexture`s.

## Run in CI

`.github/workflows/unity-postfx-render.yml` uses the
`unityci/editor:ubuntu-2022.3.20f1-focal-2` image and runs the same command.
See the workflow file for the full invocation.

## Expected output

A passing run produces an `results.xml` with seven
`PostStackShaderRenderingTests.Pass_XXX_Renders_NonBlack_Frame` cases plus the
pre-flight `VoxelTestFixture_*` tests. Failures indicate either a missing
shader (the BRP shader stripper removed a variant) or a regression in the
output of a pass (e.g. ACES producing pure black because the exposure path
sat to zero).