using System;

namespace UnityEngine
{
    /// <summary>
    /// Unity mathematics helper providing common float operations.
    /// This stub mirrors the UnityEngine API for offline compilation.
    /// </summary>
    /// <remarks>
    /// All methods delegate to <see cref="System.Math"/> with float casts.
    /// </remarks>
    public static class Mathf
    {
        /// <summary>Pi constant as a float.</summary>
        /// <value>Approximately 3.141593.</value>
        public const float PI = (float)Math.PI;

        /// <summary>
        /// Returns the sine of the specified angle.
        /// </summary>
        /// <param name="f">Angle in radians.</param>
        /// <returns>The sine of <paramref name="f"/> in the range [-1, 1].</returns>
        /// <example>
        /// <code>
        /// float y = Mathf.Sin(0f); // 0f
        /// float y90 = Mathf.Sin(Mathf.PI / 2f); // 1f
        /// </code>
        /// </example>
        public static float Sin(float f) => (float)Math.Sin(f);

        /// <summary>
        /// Returns the cosine of the specified angle.
        /// </summary>
        /// <param name="f">Angle in radians.</param>
        /// <returns>The cosine of <paramref name="f"/> in the range [-1, 1].</returns>
        /// <example>
        /// <code>
        /// float x = Mathf.Cos(0f); // 1f
        /// float x90 = Mathf.Cos(Mathf.PI / 2f); // 0f
        /// </code>
        /// </example>
        public static float Cos(float f) => (float)Math.Cos(f);

        /// <summary>
        /// Returns the square root of the specified number.
        /// </summary>
        /// <param name="f">The number to find the square root of. Must be >= 0.</param>
        /// <returns>The square root of <paramref name="f"/>.</returns>
        /// <example>
        /// <code>
        /// float root = Mathf.Sqrt(4f); // 2f
        /// </code>
        /// </example>
        public static float Sqrt(float f) => (float)Math.Sqrt(f);

        /// <summary>
        /// Clamps the given value to the range [0, 1].
        /// </summary>
        /// <param name="value">The value to clamp.</param>
        /// <returns>
        /// <paramref name="value"/> if it is in [0, 1]; otherwise 0 if value &lt; 0,
        /// or 1 if value &gt; 1.
        /// </returns>
        /// <example>
        /// <code>
        /// float clamped = Mathf.Clamp01(1.5f); // 1f
        /// </code>
        /// </example>
        public static float Clamp01(float value) => value < 0f ? 0f : value > 1f ? 1f : value;
    }

    /// <summary>
    /// Base class for all Unity objects.
    /// </summary>
    public class Object
    {
        /// <summary>
        /// Removes a GameObject, component or asset.
        /// </summary>
        /// <param name="obj">The object to destroy.</param>
        /// <remarks>
        /// In this stub the method is a no-op.
        /// </remarks>
        public static void Destroy(Object obj) { }
    }

    /// <summary>
    /// Base class for everything attached to a GameObject.
    /// </summary>
    public class Component : Object
    {
        /// <summary>
        /// Returns the component of type <typeparamref name="T"/> if the game object has one attached.
        /// </summary>
        /// <typeparam name="T">The type of component to retrieve.</typeparam>
        /// <returns>The component, or <c>null</c> if none exists.</returns>
        public T GetComponent<T>() where T : Component => null;
    }

    /// <summary>
    /// Base class for components that can be enabled or disabled.
    /// </summary>
    public class Behaviour : Component { }

    /// <summary>
    /// Base class for Unity scripts that attach to GameObjects.
    /// </summary>
    public class MonoBehaviour : Behaviour { }

    /// <summary>
    /// A render texture that can be used as a target for rendering.
    /// </summary>
    public struct RenderTexture
    {
        /// <summary>Width in pixels.</summary>
        public int width;

        /// <summary>Height in pixels.</summary>
        public int height;

        /// <summary>Descriptor containing creation parameters.</summary>
        public RenderTextureDescriptor descriptor;
    }

    /// <summary>
    /// Struct describing the properties of a render texture.
    /// </summary>
    public struct RenderTextureDescriptor
    {
        /// <summary>Width in pixels.</summary>
        public int width;

        /// <summary>Height in pixels.</summary>
        public int height;

        /// <summary>Depth buffer bits (0, 16, 24, or 32).</summary>
        public int depth;
    }

    /// <summary>
    /// Material class for controlling rendering appearance.
    /// </summary>
    public class Material
    {
        /// <summary>
        /// Creates a new material from the given shader.
        /// </summary>
        /// <param name="shader">The shader to use for this material.</param>
        public Material(Shader shader) { }

        /// <summary>
        /// Sets a float value for a property identified by ID.
        /// </summary>
        /// <param name="nameID">Property name ID.</param>
        /// <param name="value">Float value to set.</param>
        public void SetFloat(int nameID, float value) { }

        /// <summary>
        /// Sets a Vector4 value for a property identified by ID.
        /// </summary>
        /// <param name="nameID">Property name ID.</param>
        /// <param name="value">Vector4 value to set.</param>
        public void SetVector(int nameID, Vector4 value) { }

        /// <summary>
        /// Sets a texture for a property identified by ID.
        /// </summary>
        /// <param name="nameID">Property name ID.</param>
        /// <param name="tex">Texture to set.</param>
        public void SetTexture(int nameID, Texture tex) { }

        /// <summary>
        /// Checks whether the material has a property with the given name.
        /// </summary>
        /// <param name="name">The property name to check.</param>
        /// <returns><c>true</c> if the property exists; otherwise <c>false</c>.</returns>
        public bool HasProperty(string name) => false;

        /// <summary>
        /// Sets a texture for a property identified by name.
        /// </summary>
        /// <param name="name">Property name.</param>
        /// <param name="tex">Texture to set.</param>
        public void SetTexture(string name, Texture tex) { }

        /// <summary>
        /// Sets a Vector4 value for a property identified by name.
        /// </summary>
        /// <param name="name">Property name.</param>
        /// <param name="value">Vector4 value to set.</param>
        public void SetVector(string name, Vector4 value) { }

        /// <summary>
        /// Sets a float value for a property identified by name.
        /// </summary>
        /// <param name="name">Property name.</param>
        /// <param name="value">Float value to set.</param>
        public void SetFloat(string name, float value) { }
    }

    /// <summary>
    /// A shader program used for rendering.
    /// </summary>
    public class Shader
    {
        /// <summary>
        /// Finds a shader by its name.
        /// </summary>
        /// <param name="name">The name of the shader to find.</param>
        /// <returns>The shader, or <c>null</c> if not found.</returns>
        public static Shader Find(string name) => null;
    }

    /// <summary>
    /// Base class for texture objects.
    /// </summary>
    public class Texture { }

    /// <summary>
    /// 2D texture class.
    /// </summary>
    public class Texture2D : Texture { }

    /// <summary>
    /// A camera for rendering the scene.
    /// </summary>
    public class Camera
    {
        /// <summary>
        /// How and if the camera generates a depth texture.
        /// </summary>
        /// <value>A <see cref="DepthTextureMode"/> value.</value>
        public DepthTextureMode depthTextureMode;
    }

    /// <summary>
    /// Determines how the camera generates a depth texture.
    /// </summary>
    public enum DepthTextureMode
    {
        /// <summary>No depth texture is generated.</summary>
        None = 0,
        /// <summary>A depth texture is generated.</summary>
        Depth = 1,
    }

    /// <summary>
    /// Raw interface to Unity's drawing functions.
    /// </summary>
    public class Graphics
    {
        /// <summary>
        /// Copies the source texture into the destination render texture.
        /// </summary>
        /// <param name="src">Source texture.</param>
        /// <param name="dst">Destination render texture.</param>
        public static void Blit(Texture src, RenderTexture dst) { }

        /// <summary>
        /// Copies the source texture into the destination render texture using the given material.
        /// </summary>
        /// <param name="src">Source texture.</param>
        /// <param name="dst">Destination render texture.</param>
        /// <param name="mat">Material to use for rendering.</param>
        /// <param name="pass">Shader pass to use. -1 draws all passes.</param>
        public static void Blit(Texture src, RenderTexture dst, Material mat, int pass = -1) { }

        /// <summary>
        /// Copies the source texture into the destination render texture using the given material.
        /// </summary>
        /// <param name="src">Source texture.</param>
        /// <param name="dst">Destination render texture.</param>
        /// <param name="mat">Material to use for rendering.</param>
        public static void Blit(Texture src, RenderTexture dst, Material mat) { }
    }

    /// <summary>
    /// Interface for loading assets from the Resources folder.
    /// </summary>
    public class Resources
    {
        /// <summary>
        /// Loads an asset of type <typeparamref name="T"/> stored at <paramref name="path"/> in a Resources folder.
        /// </summary>
        /// <typeparam name="T">The type of asset to load.</typeparam>
        /// <param name="path">Path to the asset relative to a Resources folder.</param>
        /// <returns>The loaded asset, or <c>null</c> if not found.</returns>
        public static T Load<T>(string path) where T : Object => null;
    }

    /// <summary>
    /// Representation of four-dimensional vectors.
    /// </summary>
    public struct Vector4
    {
        /// <summary>X component of the vector.</summary>
        public float x;

        /// <summary>Y component of the vector.</summary>
        public float y;

        /// <summary>Z component of the vector.</summary>
        public float z;

        /// <summary>W component of the vector.</summary>
        public float w;

        /// <summary>
        /// Creates a new vector with the given components.
        /// </summary>
        /// <param name="x">X component.</param>
        /// <param name="y">Y component.</param>
        /// <param name="z">Z component.</param>
        /// <param name="w">W component.</param>
        public Vector4(float x, float y, float z, float w)
        {
            this.x = x;
            this.y = y;
            this.z = z;
            this.w = w;
        }
    }
}
