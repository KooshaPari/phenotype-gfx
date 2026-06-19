// Unity Test Framework (NUnit) tests for PostStack source-surface checks.

using System.IO;
using System.Text.RegularExpressions;
using NUnit.Framework;

namespace Phenotype.PostFx.Tests
{
    [TestFixture]
    public class PostStackSourceTests
    {
        private static readonly string[] ExpectedSignatures =
        {
            @"void\s+OnRenderImage\s*\(\s*RenderTexture\s+src\s*,\s*RenderTexture\s+dst\s*\)",
            @"void\s+EnsurePingPong\s*\(\s*RenderTexture\s+src\s*\)",
            @"void\s+InitMaterials\s*\(\s*\)",
            @"void\s+ReleaseMaterials\s*\(\s*\)",
            // Shader-variant validation additions
            @"interface\s+IShaderAvailabilityProvider",
            @"bool\s+IsAvailable\s*\(\s*PostFxEffect\s+effect\s*\)",
            @"void\s+SetAvailabilityProvider\s*\(\s*IShaderAvailabilityProvider\s+provider\s*\)",
            @"internal\s+void\s+ValidateShaderVariants\s*\(\s*\)",
            @"readonly\s+struct\s+PostFxPass",
            @"Material\??\s+TryLoadPass\s*\(",
            @"bool\s+ValidatePass\s*\(",
            @"bool\s+BlitIfEnabled\s*\(",
            @"public\s+bool\s+EnableVignette",
            @"public\s+bool\s+EnableChromaticAberration",
            @"PostFxEffect\.Vignette",
            @"PostFxEffect\.ChromaticAberration",
        };

        string _source = null!;

        [OneTimeSetUp]
        public void OneTimeSetUp()
        {
            string repoRoot = FindRepositoryRoot();
            string postStackPath = Path.Combine(repoRoot, "Runtime", "PostStack.cs");
            Assert.That(File.Exists(postStackPath), Is.True, $"Expected source file to exist: {postStackPath}");
            _source = File.ReadAllText(postStackPath);
        }

        static string FindRepositoryRoot()
        {
            string current = Directory.GetCurrentDirectory();

            while (!string.IsNullOrEmpty(current))
            {
                if (File.Exists(Path.Combine(current, "package.json")) &&
                    File.Exists(Path.Combine(current, "Runtime", "PostStack.cs")))
                {
                    return current;
                }

                DirectoryInfo? parent = Directory.GetParent(current);
                current = parent?.FullName ?? string.Empty;
            }

            throw new DirectoryNotFoundException("Could not locate repository root from the test executable location.");
        }

        [Test]
        public void PostStack_Contains_OnRenderImage_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[0]));
        }

        [Test]
        public void PostStack_Contains_EnsurePingPong_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[1]));
        }

        [Test]
        public void PostStack_Contains_InitMaterials_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[2]));
        }

        [Test]
        public void PostStack_Contains_ReleaseMaterials_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[3]));
        }

        [Test]
        public void PostStack_Contains_IShaderAvailabilityProvider_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[4]));
        }

        [Test]
        public void PostStack_Contains_IsAvailable_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[5]));
        }

        [Test]
        public void PostStack_Contains_SetAvailabilityProvider_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[6]));
        }

        [Test]
        public void PostStack_Contains_ValidateShaderVariants_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[7]));
        }

        [Test]
        public void PostStack_Contains_PostFxPass_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[8]));
        }

        [Test]
        public void PostStack_Contains_TryLoadPass_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[9]));
        }

        [Test]
        public void PostStack_Contains_ValidatePass_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[10]));
        }

        [Test]
        public void PostStack_Contains_BlitIfEnabled_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[11]));
        }

        [Test]
        public void PostStack_Contains_EnableVignette_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[12]));
        }

        [Test]
        public void PostStack_Contains_EnableChromaticAberration_Signature()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[13]));
        }

        [Test]
        public void PostStack_Contains_PostFxEffect_Vignette()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[14]));
        }

        [Test]
        public void PostStack_Contains_PostFxEffect_ChromaticAberration()
        {
            Assert.That(_source, Does.Match(ExpectedSignatures[15]));
        }
    }
}
