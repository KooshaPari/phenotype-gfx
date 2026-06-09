using System;

namespace UnityEngine
{
    public static class Mathf
    {
        public const float PI = (float)Math.PI;
        public static float Sin(float f) => (float)Math.Sin(f);
        public static float Cos(float f) => (float)Math.Cos(f);
        public static float Sqrt(float f) => (float)Math.Sqrt(f);
        public static float Clamp01(float value) => value < 0f ? 0f : value > 1f ? 1f : value;
    }

    public class Object { public static void Destroy(Object obj) { } }
    public class Component : Object { public T GetComponent<T>() where T : Component => null; }
    public class Behaviour : Component { }
    public class MonoBehaviour : Behaviour { }
    public struct RenderTexture { public int width, height; public RenderTextureDescriptor descriptor; }
    public struct RenderTextureDescriptor { public int width, height; public int depth; }
    public class Material { public Material(Shader shader) { } public void SetFloat(int nameID, float value) { } public void SetVector(int nameID, Vector4 value) { } public void SetTexture(int nameID, Texture tex) { } public bool HasProperty(string name) => false; public void SetTexture(string name, Texture tex) { } public void SetVector(string name, Vector4 value) { } public void SetFloat(string name, float value) { } }
    public class Shader { public static Shader Find(string name) => null; }
    public class Texture { }
    public class Texture2D : Texture { }
    public class Camera { public DepthTextureMode depthTextureMode; }
    public enum DepthTextureMode { None = 0, Depth = 1 }
    public class Graphics { public static void Blit(Texture src, RenderTexture dst) { } public static void Blit(Texture src, RenderTexture dst, Material mat, int pass = -1) { } public static void Blit(Texture src, RenderTexture dst, Material mat) { } }
    public class Resources { public static T Load<T>(string path) where T : Object => null; }
    public struct Vector4 { public float x, y, z, w; public Vector4(float x, float y, float z, float w) { this.x = x; this.y = y; this.z = z; this.w = w; } }
}
