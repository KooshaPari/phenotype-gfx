Shader "Hidden/Phenotype/BloomPass"
{
    Properties
    {
        _MainTex ("Source", 2D) = "white" {}
        _BloomTex ("Bloom", 2D) = "black" {}
        _Threshold ("Threshold", Float) = 0.8
        _Intensity ("Intensity", Float) = 0.5
    }

    SubShader
    {
        Cull Off ZWrite Off ZTest Always

        // Pass 0: Threshold extract
        Pass
        {
            Name "THRESHOLD"
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            float _Threshold;

            struct appdata { float4 vertex : POSITION; float2 uv : TEXCOORD0; };
            struct v2f { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; };

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                return o;
            }

            fixed4 frag(v2f i) : SV_Target
            {
                float3 c = tex2D(_MainTex, i.uv).rgb;
                float brightness = dot(c, float3(0.2126, 0.7152, 0.0722));
                float contrib = max(0, brightness - _Threshold);
                return fixed4(c * (contrib / max(brightness, 0.001)), 1);
            }
            ENDCG
        }

        // Pass 1: Gaussian blur (horizontal)
        Pass
        {
            Name "BLUR_H"
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma multi_compile BLOOM_LOW BLOOM_MEDIUM BLOOM_HIGH BLOOM_ULTRA
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            float4 _MainTex_TexelSize;

            struct appdata { float4 vertex : POSITION; float2 uv : TEXCOORD0; };
            struct v2f { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; };

            v2f vert(appdata v) { v2f o; o.pos = UnityObjectToClipPos(v.vertex); o.uv = v.uv; return o; }

            #if defined(BLOOM_LOW)
                static const int KERNEL_SIZE = 2;
                static const float weights[2] = { 0.5, 0.25 };
            #elif defined(BLOOM_MEDIUM)
                static const int KERNEL_SIZE = 5;
                static const float weights[5] = { 0.227027, 0.194594, 0.121622, 0.054054, 0.016216 };
            #elif defined(BLOOM_HIGH)
                static const int KERNEL_SIZE = 7;
                static const float weights[7] = { 0.227027, 0.194594, 0.121622, 0.054054, 0.016216, 0.005, 0.002 };
            #else // BLOOM_ULTRA
                static const int KERNEL_SIZE = 9;
                static const float weights[9] = { 0.227027, 0.194594, 0.121622, 0.054054, 0.016216, 0.005, 0.002, 0.001, 0.0005 };
            #endif

            fixed4 frag(v2f i) : SV_Target
            {
                float2 texel = float2(_MainTex_TexelSize.x, 0);
                float3 result = tex2D(_MainTex, i.uv).rgb * weights[0];
                for (int j = 1; j < KERNEL_SIZE; j++)
                {
                    result += tex2D(_MainTex, i.uv + texel * j).rgb * weights[j];
                    result += tex2D(_MainTex, i.uv - texel * j).rgb * weights[j];
                }
                return fixed4(result, 1);
            }
            ENDCG
        }

        // Pass 2: Gaussian blur (vertical)
        Pass
        {
            Name "BLUR_V"
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma multi_compile BLOOM_LOW BLOOM_MEDIUM BLOOM_HIGH BLOOM_ULTRA
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            float4 _MainTex_TexelSize;

            struct appdata { float4 vertex : POSITION; float2 uv : TEXCOORD0; };
            struct v2f { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; };

            v2f vert(appdata v) { v2f o; o.pos = UnityObjectToClipPos(v.vertex); o.uv = v.uv; return o; }

            #if defined(BLOOM_LOW)
                static const int KERNEL_SIZE = 2;
                static const float weights[2] = { 0.5, 0.25 };
            #elif defined(BLOOM_MEDIUM)
                static const int KERNEL_SIZE = 5;
                static const float weights[5] = { 0.227027, 0.194594, 0.121622, 0.054054, 0.016216 };
            #elif defined(BLOOM_HIGH)
                static const int KERNEL_SIZE = 7;
                static const float weights[7] = { 0.227027, 0.194594, 0.121622, 0.054054, 0.016216, 0.005, 0.002 };
            #else // BLOOM_ULTRA
                static const int KERNEL_SIZE = 9;
                static const float weights[9] = { 0.227027, 0.194594, 0.121622, 0.054054, 0.016216, 0.005, 0.002, 0.001, 0.0005 };
            #endif

            fixed4 frag(v2f i) : SV_Target
            {
                float2 texel = float2(0, _MainTex_TexelSize.y);
                float3 result = tex2D(_MainTex, i.uv).rgb * weights[0];
                for (int j = 1; j < KERNEL_SIZE; j++)
                {
                    result += tex2D(_MainTex, i.uv + texel * j).rgb * weights[j];
                    result += tex2D(_MainTex, i.uv - texel * j).rgb * weights[j];
                }
                return fixed4(result, 1);
            }
            ENDCG
        }

        // Pass 3: Composite (additive blend bloom onto source)
        Pass
        {
            Name "COMPOSITE"
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            sampler2D _BloomTex;
            float _Intensity;

            struct appdata { float4 vertex : POSITION; float2 uv : TEXCOORD0; };
            struct v2f { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; };

            v2f vert(appdata v) { v2f o; o.pos = UnityObjectToClipPos(v.vertex); o.uv = v.uv; return o; }

            fixed4 frag(v2f i) : SV_Target
            {
                float3 src = tex2D(_MainTex, i.uv).rgb;
                float3 bloom = tex2D(_BloomTex, i.uv).rgb;
                return fixed4(src + bloom * _Intensity, 1);
            }
            ENDCG
        }
    }
}
