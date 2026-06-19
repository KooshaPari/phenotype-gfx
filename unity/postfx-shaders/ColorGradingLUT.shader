// WSM3D/ColorGradingLUT
//
// 3D LUT lookup applied as a full-screen post-pass. Supports both 32-slice
// horizontal-strip LUTs (.png) and Unity Texture3D LUT assets via a single
// _LUT_Tex2D sampler — 2D strip layout (W=32×slices, H=32) decoded inline.

Shader "WSM3D/ColorGradingLUT"
{
    Properties
    {
        _MainTex ("Source", 2D) = "white" {}
        _LUT_Tex2D ("LUT (32-slice strip)", 2D) = "white" {}
        _LUT_Strength ("LUT Strength", Range(0, 1)) = 1
        _Exposure ("Exposure", Range(-4, 4)) = 0
        _Saturation ("Saturation", Range(0, 2)) = 1
    }

    SubShader
    {
        Tags { "RenderType"="Opaque" }
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            Name "ColorGradePass"

            CGPROGRAM
            #pragma vertex vert_img
            #pragma fragment frag
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            sampler2D _LUT_Tex2D;
            float _LUT_Strength, _Exposure, _Saturation;

            fixed3 ApplyLUT(fixed3 c)
            {
                c = saturate(c);
                float scale = 31.0 / 32.0;
                float offset = 0.5 / 32.0;
                float slice = c.b * 31;
                float slice0 = floor(slice);
                float slice1 = min(slice0 + 1, 31);
                float u0 = (slice0 + c.r * scale + offset) / 32;
                float u1 = (slice1 + c.r * scale + offset) / 32;
                float v = c.g * scale + offset;
                fixed3 s0 = tex2D(_LUT_Tex2D, float2(u0, v)).rgb;
                fixed3 s1 = tex2D(_LUT_Tex2D, float2(u1, v)).rgb;
                return lerp(s0, s1, slice - slice0);
            }

            fixed4 frag(v2f_img i) : SV_Target
            {
                fixed4 col = tex2D(_MainTex, i.uv);
                col.rgb *= exp2(_Exposure);
                fixed lum = dot(col.rgb, fixed3(0.299, 0.587, 0.114));
                col.rgb = lerp(fixed3(lum, lum, lum), col.rgb, _Saturation);
                fixed3 graded = ApplyLUT(col.rgb);
                col.rgb = lerp(col.rgb, graded, _LUT_Strength);
                return col;
            }
            ENDCG
        }
    }

    Fallback Off
}
