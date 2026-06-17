using System;
using System.Collections.Generic;
using Phenotype.Terrain.Materials;
using UnityEngine;
using Xunit;

namespace Phenotype.Terrain.Tests
{
    public class TerrainMaterialTests
    {
        [Fact]
        public void TerrainMaterialPropertyType_HasExpectedValues()
        {
            var values = (TerrainMaterialPropertyType[])Enum.GetValues(typeof(TerrainMaterialPropertyType));
            Assert.Contains(TerrainMaterialPropertyType.Float, values);
            Assert.Contains(TerrainMaterialPropertyType.Color, values);
            Assert.Contains(TerrainMaterialPropertyType.Texture, values);
            Assert.Contains(TerrainMaterialPropertyType.Vector, values);
        }

        [Fact]
        public void TerrainMaterialProperty_Float_ConstructorSetsType()
        {
            var prop = new TerrainMaterialProperty("Roughness", 0.75f);
            Assert.Equal("Roughness", prop.Name);
            Assert.Equal(TerrainMaterialPropertyType.Float, prop.Type);
            Assert.Equal(0.75f, prop.FloatValue);
        }

        [Fact]
        public void TerrainMaterialProperty_Color_ConstructorSetsType()
        {
            var color = new Color(0.2f, 0.4f, 0.6f, 1f);
            var prop = new TerrainMaterialProperty("Tint", color);
            Assert.Equal("Tint", prop.Name);
            Assert.Equal(TerrainMaterialPropertyType.Color, prop.Type);
            Assert.Equal(color, prop.ColorValue);
        }

        [Fact]
        public void TerrainMaterialProperty_Texture_ConstructorSetsType()
        {
            var prop = new TerrainMaterialProperty("Albedo", "textures/grass.png");
            Assert.Equal("Albedo", prop.Name);
            Assert.Equal(TerrainMaterialPropertyType.Texture, prop.Type);
            Assert.Equal("textures/grass.png", prop.TexturePath);
        }

        [Fact]
        public void TerrainMaterialProperty_Vector_ConstructorSetsType()
        {
            var vec = new Vector3(1f, 2f, 3f);
            var prop = new TerrainMaterialProperty("Offset", vec);
            Assert.Equal("Offset", prop.Name);
            Assert.Equal(TerrainMaterialPropertyType.Vector, prop.Type);
            Assert.Equal(vec, prop.VectorValue);
        }

        [Fact]
        public void TerrainMaterialProperty_EmptyName_ThrowsArgumentException()
        {
            Assert.Throws<ArgumentException>(() => new TerrainMaterialProperty("", 1f));
            Assert.Throws<ArgumentException>(() => new TerrainMaterialProperty("   ", 1f));
            Assert.Throws<ArgumentException>(() => new TerrainMaterialProperty(null, 1f));
        }

        [Fact]
        public void TerrainMaterialProperty_FloatValue_OnNonFloatType_Throws()
        {
            var prop = new TerrainMaterialProperty("Tint", Color.red);
            Assert.Throws<InvalidOperationException>(() => prop.FloatValue);
            Assert.Throws<InvalidOperationException>(() => prop.FloatValue = 1f);
        }

        [Fact]
        public void TerrainMaterialProperty_ColorValue_OnNonColorType_Throws()
        {
            var prop = new TerrainMaterialProperty("Roughness", 0.5f);
            Assert.Throws<InvalidOperationException>(() => prop.ColorValue);
            Assert.Throws<InvalidOperationException>(() => prop.ColorValue = Color.red);
        }

        [Fact]
        public void TerrainMaterialProperty_TexturePath_OnNonTextureType_Throws()
        {
            var prop = new TerrainMaterialProperty("Roughness", 0.5f);
            Assert.Throws<InvalidOperationException>(() => prop.TexturePath);
            Assert.Throws<InvalidOperationException>(() => prop.TexturePath = "foo.png");
        }

        [Fact]
        public void TerrainMaterialProperty_VectorValue_OnNonVectorType_Throws()
        {
            var prop = new TerrainMaterialProperty("Roughness", 0.5f);
            Assert.Throws<InvalidOperationException>(() => prop.VectorValue);
            Assert.Throws<InvalidOperationException>(() => prop.VectorValue = Vector3.one);
        }

        [Fact]
        public void TerrainMaterialProperty_FloatValue_CanBeUpdated()
        {
            var prop = new TerrainMaterialProperty("Roughness", 0.5f);
            prop.FloatValue = 0.9f;
            Assert.Equal(0.9f, prop.FloatValue);
        }

        [Fact]
        public void TerrainMaterialProperty_ColorValue_CanBeUpdated()
        {
            var prop = new TerrainMaterialProperty("Tint", Color.white);
            prop.ColorValue = Color.black;
            Assert.Equal(Color.black, prop.ColorValue);
        }

        [Fact]
        public void TerrainMaterialProperty_TexturePath_CanBeUpdated()
        {
            var prop = new TerrainMaterialProperty("Albedo", "old.png");
            prop.TexturePath = "new.png";
            Assert.Equal("new.png", prop.TexturePath);
        }

        [Fact]
        public void TerrainMaterialProperty_TexturePath_NullSetBecomesEmpty()
        {
            var prop = new TerrainMaterialProperty("Albedo", "path.png");
            prop.TexturePath = null;
            Assert.Equal(string.Empty, prop.TexturePath);
        }

        [Fact]
        public void TerrainMaterialProperty_VectorValue_CanBeUpdated()
        {
            var prop = new TerrainMaterialProperty("Offset", Vector3.zero);
            prop.VectorValue = new Vector3(10f, 0f, 10f);
            Assert.Equal(new Vector3(10f, 0f, 10f), prop.VectorValue);
        }

        [Fact]
        public void TerrainMaterial_Constructor_SetsNameAndGuid()
        {
            var mat = new TerrainMaterial("Grass");
            Assert.Equal("Grass", mat.Name);
            Assert.NotEqual(Guid.Empty, mat.Id);
        }

        [Fact]
        public void TerrainMaterial_EmptyName_ThrowsArgumentException()
        {
            Assert.Throws<ArgumentException>(() => new TerrainMaterial(""));
            Assert.Throws<ArgumentException>(() => new TerrainMaterial("   "));
            Assert.Throws<ArgumentException>(() => new TerrainMaterial(null));
        }

        [Fact]
        public void TerrainMaterial_DefaultValues_AreExpected()
        {
            var mat = new TerrainMaterial("Test");
            Assert.Equal(Color.white, mat.BaseColor);
            Assert.Equal(string.Empty, mat.MainTexturePath);
            Assert.Equal(string.Empty, mat.NormalMapPath);
            Assert.Equal(1f, mat.TextureScale);
            Assert.Equal(0.5f, mat.Smoothness);
            Assert.Equal(0f, mat.Metallic);
            Assert.Equal(0, mat.PropertyCount);
        }

        [Fact]
        public void TerrainMaterial_AddProperty_IncreasesCount()
        {
            var mat = new TerrainMaterial("Test");
            mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.5f));
            Assert.Equal(1, mat.PropertyCount);
            Assert.True(mat.HasProperty("Roughness"));
        }

        [Fact]
        public void TerrainMaterial_AddProperty_DuplicateName_Throws()
        {
            var mat = new TerrainMaterial("Test");
            mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.5f));
            Assert.Throws<ArgumentException>(() => mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.9f)));
        }

        [Fact]
        public void TerrainMaterial_AddProperty_Null_ThrowsArgumentNullException()
        {
            var mat = new TerrainMaterial("Test");
            Assert.Throws<ArgumentNullException>(() => mat.AddProperty(null));
        }

        [Fact]
        public void TerrainMaterial_GetProperty_ReturnsCorrectProperty()
        {
            var mat = new TerrainMaterial("Test");
            var prop = new TerrainMaterialProperty("Roughness", 0.5f);
            mat.AddProperty(prop);
            Assert.Equal(prop, mat.GetProperty("Roughness"));
        }

        [Fact]
        public void TerrainMaterial_GetProperty_Missing_ThrowsKeyNotFoundException()
        {
            var mat = new TerrainMaterial("Test");
            Assert.Throws<KeyNotFoundException>(() => mat.GetProperty("Missing"));
        }

        [Fact]
        public void TerrainMaterial_TryGetProperty_ReturnsTrueWhenFound()
        {
            var mat = new TerrainMaterial("Test");
            mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.5f));
            bool found = mat.TryGetProperty("Roughness", out var prop);
            Assert.True(found);
            Assert.NotNull(prop);
            Assert.Equal("Roughness", prop.Name);
        }

        [Fact]
        public void TerrainMaterial_TryGetProperty_ReturnsFalseWhenMissing()
        {
            var mat = new TerrainMaterial("Test");
            bool found = mat.TryGetProperty("Missing", out var prop);
            Assert.False(found);
            Assert.Null(prop);
        }

        [Fact]
        public void TerrainMaterial_HasProperty_IsCaseInsensitive()
        {
            var mat = new TerrainMaterial("Test");
            mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.5f));
            Assert.True(mat.HasProperty("roughness"));
            Assert.True(mat.HasProperty("ROUGHNESS"));
        }

        [Fact]
        public void TerrainMaterial_RemoveProperty_RemovesAndReturnsTrue()
        {
            var mat = new TerrainMaterial("Test");
            mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.5f));
            bool removed = mat.RemoveProperty("Roughness");
            Assert.True(removed);
            Assert.Equal(0, mat.PropertyCount);
        }

        [Fact]
        public void TerrainMaterial_RemoveProperty_Missing_ReturnsFalse()
        {
            var mat = new TerrainMaterial("Test");
            bool removed = mat.RemoveProperty("Missing");
            Assert.False(removed);
        }

        [Fact]
        public void TerrainMaterial_PropertyNames_ReturnsAllNames()
        {
            var mat = new TerrainMaterial("Test");
            mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.5f));
            mat.AddProperty(new TerrainMaterialProperty("Metallic", 0.1f));
            var names = mat.PropertyNames;
            Assert.Contains("Roughness", names);
            Assert.Contains("Metallic", names);
            Assert.Equal(2, names.Count);
        }

        [Fact]
        public void TerrainMaterial_MultipleProperties_CanBeMixedTypes()
        {
            var mat = new TerrainMaterial("Test");
            mat.AddProperty(new TerrainMaterialProperty("Roughness", 0.5f));
            mat.AddProperty(new TerrainMaterialProperty("Tint", Color.red));
            mat.AddProperty(new TerrainMaterialProperty("Albedo", "grass.png"));
            mat.AddProperty(new TerrainMaterialProperty("WindDir", Vector3.up));

            Assert.Equal(TerrainMaterialPropertyType.Float, mat.GetProperty("Roughness").Type);
            Assert.Equal(TerrainMaterialPropertyType.Color, mat.GetProperty("Tint").Type);
            Assert.Equal(TerrainMaterialPropertyType.Texture, mat.GetProperty("Albedo").Type);
            Assert.Equal(TerrainMaterialPropertyType.Vector, mat.GetProperty("WindDir").Type);
        }

        [Fact]
        public void TerrainMaterial_MaterialName_CanBeChanged()
        {
            var mat = new TerrainMaterial("OldName");
            mat.Name = "NewName";
            Assert.Equal("NewName", mat.Name);
        }
    }
}
