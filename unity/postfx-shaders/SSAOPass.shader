// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

// Hidden/Phenotype/SSAOPass
//
// Screen-space ambient occlusion via depth-buffer sampling.  Uses a
// configurable sample kernel (up to 64 samples) passed from the C# pass.
// Output: full-screen RGBA occlusion mask (rgb *= occlusion).

Shader "Hidden/Phenotype/SSAOPass"
{
    Properties
    {
        _MainTex ("Source", 2D) = "white" {}
        _Radius ("AO Radius", Float) = 0.5
        _Intensity ("AO Intensity", Float) = 1.2
        _Bias ("AO Bias", Float) = 0.04
        _SampleCount ("Sample Count", Int) = 8
    }

    SubShader
    {
        Tags { "RenderType"="Opaque" }
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            Name "SSAOPASS"

            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma multi_compile SSAOPASS
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            sampler2D _CameraDepthTexture;
            float4 _MainTex_TexelSize;
            float _Radius;
            float _Intensity;
            float _Bias;
            int _SampleCount;
            float4 _Samples[64];

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
            };

            struct v2f
            {
                float4 pos : SV_POSITION;
                float2 uv : TEXCOORD0;
            };

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                return o;
            }

            float SampleLinearDepth(float2 uv)
            {
                float raw = SAMPLE_DEPTH_TEXTURE(_CameraDepthTexture, uv);
                return Linear01Depth(raw);
            }

            fixed4 frag(v2f i) : SV_Target
            {
                float centerDepth = SampleLinearDepth(i.uv);
                if (centerDepth >= 0.999)
                    return tex2D(_MainTex, i.uv);

                float occlusion = 0.0;
                float2 ts = _MainTex_TexelSize.xy * _Radius * 32.0;

                for (int k = 0; k < _SampleCount; k++)
                {
                    float2 off = _Samples[k].xy * _Samples[k].w * ts;
                    float d = SampleLinearDepth(i.uv + off);
                    float diff = max(centerDepth - d - _Bias, 0.0);
                    occlusion += saturate(diff * 6.0);
                }

                occlusion = saturate(1.0 - occlusion * _Intensity / (float)_SampleCount);
                fixed4 col = tex2D(_MainTex, i.uv);
                col.rgb *= occlusion;
                return col;
            }
            ENDCG
        }
    }

    Fallback Off
}
