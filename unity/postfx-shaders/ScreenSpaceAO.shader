// WSM3D/ScreenSpaceAO
//
// Screen-space ambient occlusion via depth-buffer sampling. 8-tap rotated
// kernel. Output: full-screen R8 occlusion mask (0=fully occluded, 1=open).
// Composite by multiplying scene color by occlusion in OnRenderImage pipeline.

Shader "WSM3D/ScreenSpaceAO"
{
    Properties
    {
        _MainTex ("Source", 2D) = "white" {}
        _Radius ("AO Radius (world units)", Range(0.01, 5)) = 0.5
        _Intensity ("AO Intensity", Range(0, 4)) = 1.2
        _Bias ("AO Bias", Range(0, 0.5)) = 0.04
    }

    SubShader
    {
        Tags { "RenderType"="Opaque" }
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            Name "SSAOPass"

            CGPROGRAM
            #pragma vertex vert_img
            #pragma fragment frag
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            sampler2D _CameraDepthTexture;
            float4 _MainTex_TexelSize;
            float _Radius, _Intensity, _Bias;

            static const float2 kKernel[8] = {
                float2( 0.707,  0.000), float2( 0.354,  0.612),
                float2( 0.000,  0.707), float2(-0.354,  0.612),
                float2(-0.707,  0.000), float2(-0.354, -0.612),
                float2( 0.000, -0.707), float2( 0.354, -0.612)
            };

            float SampleLinearDepth(float2 uv)
            {
                float raw = SAMPLE_DEPTH_TEXTURE(_CameraDepthTexture, uv);
                return Linear01Depth(raw);
            }

            fixed4 frag(v2f_img i) : SV_Target
            {
                fixed4 col = tex2D(_MainTex, i.uv);
                float centerDepth = SampleLinearDepth(i.uv);
                if (centerDepth >= 0.999) return col;
                float occlusion = 0;
                float2 ts = _MainTex_TexelSize.xy * _Radius * 32;
                [unroll] for (int k = 0; k < 8; k++)
                {
                    float2 off = kKernel[k] * ts;
                    float d = SampleLinearDepth(i.uv + off);
                    float diff = max(centerDepth - d - _Bias, 0);
                    occlusion += saturate(diff * 6);
                }
                occlusion = saturate(1 - occlusion * _Intensity * 0.125);
                col.rgb *= occlusion;
                return col;
            }
            ENDCG
        }
    }

    Fallback Off
}
