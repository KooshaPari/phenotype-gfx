using System;

namespace UnityEngine
{
    public struct Vector2
    {
        public float x, y;
        public static readonly Vector2 right = new Vector2(1f, 0f);
        public static readonly Vector2 one = new Vector2(1f, 1f);
        public static readonly Vector2 zero = new Vector2(0f, 0f);
        public float sqrMagnitude => x * x + y * y;
        public float magnitude => (float)Math.Sqrt(sqrMagnitude);
        public Vector2 normalized => magnitude > 1e-10f ? this / magnitude : right;
        public Vector2(float x, float y) { this.x = x; this.y = y; }
        public static Vector2 operator +(Vector2 a, Vector2 b) => new Vector2(a.x + b.x, a.y + b.y);
        public static Vector2 operator -(Vector2 a, Vector2 b) => new Vector2(a.x - b.x, a.y - b.y);
        public static Vector2 operator *(Vector2 a, float d) => new Vector2(a.x * d, a.y * d);
        public static Vector2 operator /(Vector2 a, float d) => new Vector2(a.x / d, a.y / d);
    }

    public struct Vector3
    {
        public float x, y, z;
        public static readonly Vector3 up = new Vector3(0f, 1f, 0f);
        public float sqrMagnitude => x * x + y * y + z * z;
        public float magnitude => (float)Math.Sqrt(sqrMagnitude);
        public Vector3 normalized => magnitude > 1e-10f ? this / magnitude : up;
        public Vector3(float x, float y, float z) { this.x = x; this.y = y; this.z = z; }
        public static Vector3 operator +(Vector3 a, Vector3 b) => new Vector3(a.x + b.x, a.y + b.y, a.z + b.z);
        public static Vector3 operator -(Vector3 a, Vector3 b) => new Vector3(a.x - b.x, a.y - b.y, a.z - b.z);
        public static Vector3 operator *(Vector3 a, float d) => new Vector3(a.x * d, a.y * d, a.z * d);
        public static Vector3 operator /(Vector3 a, float d) => new Vector3(a.x / d, a.y / d, a.z / d);
        public static Vector3 Cross(Vector3 a, Vector3 b) => new Vector3(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x);
    }

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
