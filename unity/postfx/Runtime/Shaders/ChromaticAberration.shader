Shader "Hidden/WSM3D/ChromaticAberration"
{
    Properties
    {
        _MainTex ("Source", 2D) = "white" {}
        _Intensity ("Intensity", Range(0, 1)) = 0.15
    }

    SubShader
    {
        Cull Off ZWrite Off ZTest Always

        Pass
        {
            Name "CHROMATIC_ABERRATION"

            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"

            sampler2D _MainTex;
            float _Intensity;

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
                float2 centered = i.uv - 0.5;
                float dist = length(centered);
                float2 direction = centered / max(dist, 0.0001);
                float shift = _Intensity * dist * 0.03;
                float2 offset = direction * shift;

                fixed3 color;
                color.r = tex2D(_MainTex, i.uv + offset).r;
                color.g = tex2D(_MainTex, i.uv).g;
                color.b = tex2D(_MainTex, i.uv - offset).b;
                return fixed4(color, 1.0);
            }
            ENDCG
        }
    }

    Fallback Off
}
