// Extra Unity stubs for BenchmarkDotNet project — types not in the base UnityStubs.cs
// but required by IPostFxPass.cs and other newer runtime files.

#pragma warning disable CS8618
#pragma warning disable IDE0060

namespace UnityEngine
{
    public struct Color
    {
        public float r, g, b, a;
        public Color(float r, float g, float b, float a = 1f)
        {
            this.r = r; this.g = g; this.b = b; this.a = a;
        }
    }
}
