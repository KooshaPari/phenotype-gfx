> 📋 Read [`CONTRIBUTING.md`](./CONTRIBUTING.md) and [`AGENTS.md`](./AGENTS.md) before opening this PR. Non-trivial features require a linked AgilePlus spec.

## Summary

<!-- What does this PR do? -->

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Security fix

## Affected surface

- [ ] `HeightField` storage
- [ ] `ChunkMeshBuilder` mesh generation
- [ ] `TerrainLod` LOD selection
- [ ] Consumer API (sibling project references)
- [ ] Docs / screenshots

## Testing

- [ ] `dotnet build phenotype-terrain.csproj -c Release`
- [ ] In-repo consumer `phenotype-water` recompiles cleanly
- [ ] Unity test scene renders without warnings (if applicable)

## Spec / Traceability

<!-- Link the AgilePlus spec, FR IDs, or ADR that this change implements -->
- Spec:
- FR / NFR:

## Risks & Rollback

<!-- Known risks, breaking changes, and how to roll back if needed. Note that public API changes must be backward-compatible; add overloads before deprecating/removing. -->

## Related

<!-- Issues this PR closes; PRs/specs this depends on -->
Closes #
