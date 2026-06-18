using BenchmarkDotNet.Running;

namespace Phenotype.PostFx.Benchmarks;

class Program
{
    static void Main(string[] args)
    {
        BenchmarkSwitcher.FromAssembly(typeof(Program).Assembly).Run(args);
    }
}
