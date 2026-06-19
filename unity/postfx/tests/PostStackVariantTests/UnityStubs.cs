// Minimal Unity Engine stubs so PostStack.cs can compile and be tested
// in a pure .NET environment without the UnityEngine assembly.
//
// These stubs replicate only the surface area that PostStack.cs touches.
// Debug.LogWarning is wired to a thread-local capture list so tests can
// assert on warnings without coupling to System.Console.

using System;
using System.Collections.Generic;

#pragma warning disable CS8618  // Non-nullable field uninitialized (stubs)
#pragma warning disable IDE0060 // Unused parameters (stubs)

// ReSharper disable all

namespace UnityEngine
{
    // -----------------------------------------------------------------------
    // Capture list for Debug.LogWarning — tests read via DebugCapture.Warnings
    // -----------------------------------------------------------------------
    public static class DebugCapture
    {
        [ThreadStatic]
        private static List<string>? _warnings;

        public static IReadOnlyList<string> Warnings
            => _warnings ??= new List<string>();

        internal static void Add(string msg)
            => (_warnings ??= new List<string>()).Add(msg);

        public static void Clear()
            => (_warnings ??= new List<string>()).Clear();
    }

    public static class GraphicsCapture
    {
        public sealed record BlitCall(RenderTexture? Src, RenderTexture? Dst, Material? Material, int? Pass);

        [ThreadStatic]
        private static List<BlitCall>? _blits;

        public static IReadOnlyList<BlitCall> Blits
            => _blits ??= new List<BlitCall>();

        internal static void Add(RenderTexture src, RenderTexture dst, Material? material, int? pass)
            => (_blits ??= new List<BlitCall>()).Add(new BlitCall(src, dst, material, pass));

        public static void Clear()
            => (_blits ??= new List<BlitCall>()).Clear();
    }

    // -----------------------------------------------------------------------
    // UnityEngine types
    // -----------------------------------------------------------------------

    public class Object
    {
        public static void Destroy(Object obj) { }
        public static void DestroyImmediate(Object obj) { }
    }

    public class Component : Object
    {
        // GetComponent<T>() — returns null in stubs (no scene graph)
        public T? GetComponent<T>() where T : Component => null;
    }

    public class Behaviour : Component { }

    public class MonoBehaviour : Behaviour { }

    public class Transform : Component { }

    public class GameObject : Object
    {
        public string name { get; set; } = "GameObject";
        public Transform transform { get; } = new Transform();

        public GameObject() { }
        public GameObject(string name) { this.name = name; }

        public T AddComponent<T>() where T : Component => (T)Activator.CreateInstance(typeof(T))!;
        public T? GetComponent<T>() where T : Component => null;
    }

    public struct Vector2
    {
        public float x, y;
        public static readonly Vector2 right = new Vector2 { x = 1 };
        public static readonly Vector2 one = new Vector2 { x = 1, y = 1 };
        public Vector2(float x, float y)
        {
            this.x = x;
            this.y = y;
        }
        public float sqrMagnitude => x * x + y * y;
        public void Normalize()
        {
            float mag = (float)System.Math.Sqrt(x * x + y * y);
            if (mag > 0) { x /= mag; y /= mag; }
        }
    }

    public struct Vector4
    {
        public float x, y, z, w;
        public Vector4(float x, float y, float z, float w)
        { this.x = x; this.y = y; this.z = z; this.w = w; }
    }

    public class Texture : Object
    {
        public static implicit operator bool(Texture? t) => t is not null;
    }

    public class Texture2D : Texture
    {
        public static implicit operator bool(Texture2D? t) => t is not null;
    }

    public class RenderTexture : Texture
    {
        public new int width;
        public int height;
        public RenderTextureDescriptor descriptor => new RenderTextureDescriptor();
        public RenderTextureFormat format => RenderTextureFormat.ARGB32;

        public static RenderTexture GetTemporary(int w, int h, int depth, RenderTextureFormat fmt)
            => new RenderTexture { width = w, height = h };
        public static RenderTexture GetTemporary(RenderTextureDescriptor desc)
            => new RenderTexture();
        public static void ReleaseTemporary(RenderTexture rt) { }

        public static implicit operator bool(RenderTexture? rt) => rt is not null;
    }

    public struct RenderTextureDescriptor { }

    public enum RenderTextureFormat { ARGB32 }

    public enum DepthTextureMode { None = 0, Depth = 1 }

    public class Camera : Component
    {
        public DepthTextureMode depthTextureMode { get; set; }
    }

    public class Shader : Object
    {
        public static int PropertyToID(string name) => name.GetHashCode();
        public static Shader? Find(string name) => null;
    }

    public class Material : Object
    {
        private readonly System.Collections.Generic.HashSet<string> _keywords = new();
        public Material(Shader s) { }
        public bool HasProperty(string name) => false;
        public bool HasProperty(int id) => false;
        public void SetTexture(string name, Texture? tex) { }
        public void SetTexture(int id, Texture? tex) { }
        public void SetTexture(string name, RenderTexture? tex) { }
        public void SetTexture(int id, RenderTexture? tex) { }
        public void SetVector(string name, Vector4 v) { }
        public void SetVector(int id, Vector4 v) { }
        public void SetInt(string name, int v) { }
        public void SetInt(int id, int v) { }
        public void SetFloat(string name, float v) { }
        public void SetFloat(int id, float v) { }
        public void SetVectorArray(string name, Vector4[] arr) { }
        public void SetVectorArray(int id, Vector4[] arr) { }

        public void EnableKeyword(string keyword) => _keywords.Add(keyword);
        public void DisableKeyword(string keyword) => _keywords.Remove(keyword);
        public bool IsKeywordEnabled(string keyword) => _keywords.Contains(keyword);

        // Unity Material has an implicit bool conversion (non-null = true)
        public static implicit operator bool(Material? m) => m is not null;
    }

    public class MaterialPropertyBlock : Object
    {
    }

    public static class Resources
    {
        public static T? Load<T>(string path) where T : Object => null;
    }

    public static class Graphics
    {
        public static void Blit(RenderTexture src, RenderTexture dst)
            => GraphicsCapture.Add(src, dst, null, null);
        public static void Blit(RenderTexture src, RenderTexture dst, Material mat)
            => GraphicsCapture.Add(src, dst, mat, null);
        public static void Blit(RenderTexture src, RenderTexture dst, Material mat, int pass)
            => GraphicsCapture.Add(src, dst, mat, pass);
    }

    public static class Mathf
    {
        public static float Lerp(float a, float b, float t) => a + (b - a) * t;
        public static int Max(int a, int b) => System.Math.Max(a, b);
        public static float Max(float a, float b) => System.Math.Max(a, b);
    }

    public enum LogType { Error, Assert, Warning, Log, Exception }

    public interface ILogHandler
    {
        void LogFormat(LogType logType, Object context, string format, params object[] args);
        void LogException(Exception exception, Object context);
    }

    public class DefaultLogHandler : ILogHandler
    {
        public void LogFormat(LogType logType, Object context, string format, params object[] args) { }
        public void LogException(Exception exception, Object context) { }
    }

    public class Logger
    {
        public ILogHandler logHandler { get; set; } = new DefaultLogHandler();
    }

    public static class Debug
    {
        public static Logger unityLogger { get; } = new Logger { logHandler = new DefaultLogHandler() };

        public static void LogWarning(object msg)
        {
            DebugCapture.Add(msg?.ToString() ?? string.Empty);
            Application.LogMessageReceived(msg?.ToString() ?? string.Empty, "", LogType.Warning);
            unityLogger.logHandler?.LogFormat(LogType.Warning, null, "{0}", msg);
        }

        public static void Log(object msg) { }
        public static void LogError(object msg) { }
    }

    public static class Application
    {
        public static event Action<string, string, LogType>? logMessageReceived;
        public static event Action<string, string, LogType>? logMessageReceivedThreaded;

        internal static void LogMessageReceived(string condition, string stacktrace, LogType type)
        {
            logMessageReceived?.Invoke(condition, stacktrace, type);
            logMessageReceivedThreaded?.Invoke(condition, stacktrace, type);
        }
    }

    // -----------------------------------------------------------------------
    // Attribute stubs
    // -----------------------------------------------------------------------

    [System.AttributeUsage(System.AttributeTargets.Field)]
    public sealed class HeaderAttribute : System.Attribute
    {
        public HeaderAttribute(string header) { }
    }
}
