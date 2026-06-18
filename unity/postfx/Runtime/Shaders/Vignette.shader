Shader "Hidden/WSM3D/Vignette"
{
    Properties
    {
        _MainTex ("Source", 2D) = "white" {}
        _Center ("Center", Vector) = (0.5, 0.5, 0, 0)
        _Intensity ("Intensity", Range(0, 1)) = 0.45
        _Smoothness ("Smoothness", Range(0, 1)) = 0.6
        _Roundness ("Roundness", Range(0.1, 2)) = 1
    }

    SubShader
    {
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            Name "VIGNETTE"

            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            float4 _Center;
            float _Intensity;
            float _Smoothness;
            float _Roundness;

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

            fixed4 frag(v2f i) : SV_Target
            {
                float3 color = tex2D(_MainTex, i.uv).rgb;
                float2 p = i.uv - _Center.xy;
                p.x *= _Roundness;
                p.y /= max(_Roundness, 0.0001);

                float dist = length(p);
                float inner = saturate(1.0 - _Smoothness);
                float vignette = 1.0 - smoothstep(inner, 1.0, dist);
                color *= lerp(1.0, vignette, _Intensity);
                return fixed4(color, 1.0);
            }
            ENDCG
        }
    }

    Fallback Off
}
