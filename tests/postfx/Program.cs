using System.Text.RegularExpressions;

static class Program
{
    private static readonly string[] ExpectedSignatures =
    [
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
    ];

    public static int Main()
    {
        try
        {
            string repoRoot = FindRepositoryRoot();
            string postStackPath = Path.Combine(repoRoot, "Runtime", "PostStack.cs");

            Assert(File.Exists(postStackPath), $"Expected source file to exist: {postStackPath}");

            string source = File.ReadAllText(postStackPath);
            foreach (string signature in ExpectedSignatures)
            {
                Assert(
                    Regex.IsMatch(source, signature, RegexOptions.CultureInvariant | RegexOptions.Multiline),
                    $"Expected PostStack.cs to contain signature matching: {signature}");
            }

            Console.WriteLine("PostStack source checks passed.");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine(ex.Message);
            return 1;
        }
    }

    private static string FindRepositoryRoot()
    {
        string? current = AppContext.BaseDirectory;

        while (!string.IsNullOrEmpty(current))
        {
            if (File.Exists(Path.Combine(current, "package.json")) &&
                File.Exists(Path.Combine(current, "Runtime", "PostStack.cs")))
            {
                return current;
            }

            DirectoryInfo? parent = Directory.GetParent(current);
            current = parent?.FullName;
        }

        throw new DirectoryNotFoundException("Could not locate repository root from the test executable location.");
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }
}
