#!/usr/bin/env python3
"""Generate panel.glb: a 1m x 1m x 0.008m box with front face UV-mapped."""

import json
import struct
import os

PANEL_WIDTH = 1.0
PANEL_HEIGHT = 1.0
PANEL_DEPTH = 0.008
HALF_W = PANEL_WIDTH / 2.0
HALF_H = PANEL_HEIGHT / 2.0
HALF_D = PANEL_DEPTH / 2.0  # 0.004

# Each face: list of (position, normal, uv) per vertex, then 2 triangles (indices into that face's 4 verts)
# Vertex order: bottom-left, bottom-right, top-right, top-left (when facing the face)

def make_face(positions, normal, uvs):
    """Return (vertices, indices) for a quad face. vertices = list of (pos, normal, uv)."""
    verts = [(positions[i], normal, uvs[i]) for i in range(4)]
    # Two triangles: 0,1,2 and 0,2,3
    return verts, [0, 1, 2, 0, 2, 3]

# Front face: z = +HALF_D, facing +Z
# Viewed from front: bottom-left=(-W,-H,+D), bottom-right=(+W,-H,+D), top-right=(+W,+H,+D), top-left=(-W,+H,+D)
front_verts, front_idx = make_face(
    [(-HALF_W, -HALF_H, HALF_D), (HALF_W, -HALF_H, HALF_D), (HALF_W, HALF_H, HALF_D), (-HALF_W, HALF_H, HALF_D)],
    (0.0, 0.0, 1.0),
    [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)],  # flip V: gltf UV origin is top-left
)

# Back face: z = -HALF_D, facing -Z
back_verts, back_idx = make_face(
    [(HALF_W, -HALF_H, -HALF_D), (-HALF_W, -HALF_H, -HALF_D), (-HALF_W, HALF_H, -HALF_D), (HALF_W, HALF_H, -HALF_D)],
    (0.0, 0.0, -1.0),
    [(0.0, 0.0)] * 4,
)

# Left face: x = -HALF_W, facing -X
left_verts, left_idx = make_face(
    [(-HALF_W, -HALF_H, -HALF_D), (-HALF_W, -HALF_H, HALF_D), (-HALF_W, HALF_H, HALF_D), (-HALF_W, HALF_H, -HALF_D)],
    (-1.0, 0.0, 0.0),
    [(0.0, 0.0)] * 4,
)

# Right face: x = +HALF_W, facing +X
right_verts, right_idx = make_face(
    [(HALF_W, -HALF_H, HALF_D), (HALF_W, -HALF_H, -HALF_D), (HALF_W, HALF_H, -HALF_D), (HALF_W, HALF_H, HALF_D)],
    (1.0, 0.0, 0.0),
    [(0.0, 0.0)] * 4,
)

# Top face: y = +HALF_H, facing +Y
top_verts, top_idx = make_face(
    [(-HALF_W, HALF_H, HALF_D), (HALF_W, HALF_H, HALF_D), (HALF_W, HALF_H, -HALF_D), (-HALF_W, HALF_H, -HALF_D)],
    (0.0, 1.0, 0.0),
    [(0.0, 0.0)] * 4,
)

# Bottom face: y = -HALF_H, facing -Y
bottom_verts, bottom_idx = make_face(
    [(-HALF_W, -HALF_H, -HALF_D), (HALF_W, -HALF_H, -HALF_D), (HALF_W, -HALF_H, HALF_D), (-HALF_W, -HALF_H, HALF_D)],
    (0.0, -1.0, 0.0),
    [(0.0, 0.0)] * 4,
)

all_faces = [front_verts, back_verts, left_verts, right_verts, top_verts, bottom_verts]
all_idx_lists = [front_idx, back_idx, left_idx, right_idx, top_idx, bottom_idx]

# Flatten all vertices and compute global indices
positions = []
normals = []
texcoords = []
indices = []

base_vertex = 0
for face_verts, face_idx in zip(all_faces, all_idx_lists):
    for pos, norm, uv in face_verts:
        positions.append(pos)
        normals.append(norm)
        texcoords.append(uv)
    indices.extend([base_vertex + i for i in face_idx])
    base_vertex += len(face_verts)

num_vertices = len(positions)
num_indices = len(indices)

# Compute bounding box for positions accessor
pos_min = [min(p[i] for p in positions) for i in range(3)]
pos_max = [max(p[i] for p in positions) for i in range(3)]

# Pack binary data
# Layout: indices (uint16, padded to 4 bytes) | positions (vec3 float32) | normals (vec3 float32) | texcoords (vec2 float32)

def pad4(data):
    remainder = len(data) % 4
    if remainder:
        data += b'\x00' * (4 - remainder)
    return data

indices_bytes = struct.pack(f'<{num_indices}H', *indices)
indices_bytes = pad4(indices_bytes)

positions_bytes = b''.join(struct.pack('<fff', *p) for p in positions)
normals_bytes = b''.join(struct.pack('<fff', *n) for n in normals)
texcoords_bytes = b''.join(struct.pack('<ff', *uv) for uv in texcoords)

indices_offset = 0
indices_length = len(indices_bytes)
positions_offset = indices_offset + indices_length
positions_length = len(positions_bytes)
normals_offset = positions_offset + positions_length
normals_length = len(normals_bytes)
texcoords_offset = normals_offset + normals_length
texcoords_length = len(texcoords_bytes)

bin_data = indices_bytes + positions_bytes + normals_bytes + texcoords_bytes
bin_length = len(bin_data)

# Build glTF JSON
gltf = {
    "asset": {"version": "2.0", "generator": "build_panel.py"},
    "scene": 0,
    "scenes": [{"nodes": [0]}],
    "nodes": [{"mesh": 0}],
    "meshes": [
        {
            "name": "panel",
            "primitives": [
                {
                    "attributes": {
                        "POSITION": 1,
                        "NORMAL": 2,
                        "TEXCOORD_0": 3,
                    },
                    "indices": 0,
                    "material": 0,
                }
            ],
        }
    ],
    "materials": [
        {
            "name": "panel_material",
            "pbrMetallicRoughness": {
                "baseColorFactor": [0.15, 0.15, 0.15, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0,
            },
            "doubleSided": False,
        }
    ],
    "accessors": [
        # 0: indices
        {
            "bufferView": 0,
            "byteOffset": 0,
            "componentType": 5123,  # UNSIGNED_SHORT
            "count": num_indices,
            "type": "SCALAR",
        },
        # 1: positions
        {
            "bufferView": 1,
            "byteOffset": 0,
            "componentType": 5126,  # FLOAT
            "count": num_vertices,
            "type": "VEC3",
            "min": pos_min,
            "max": pos_max,
        },
        # 2: normals
        {
            "bufferView": 2,
            "byteOffset": 0,
            "componentType": 5126,
            "count": num_vertices,
            "type": "VEC3",
        },
        # 3: texcoords
        {
            "bufferView": 3,
            "byteOffset": 0,
            "componentType": 5126,
            "count": num_vertices,
            "type": "VEC2",
        },
    ],
    "bufferViews": [
        # 0: indices
        {"buffer": 0, "byteOffset": indices_offset, "byteLength": indices_length, "target": 34963},
        # 1: positions
        {"buffer": 0, "byteOffset": positions_offset, "byteLength": positions_length, "target": 34962},
        # 2: normals
        {"buffer": 0, "byteOffset": normals_offset, "byteLength": normals_length, "target": 34962},
        # 3: texcoords
        {"buffer": 0, "byteOffset": texcoords_offset, "byteLength": texcoords_length, "target": 34962},
    ],
    "buffers": [{"byteLength": bin_length}],
}

json_bytes = json.dumps(gltf, separators=(',', ':')).encode('utf-8')
json_bytes = pad4(json_bytes)

# GLB assembly
# Header: magic(4) + version(4) + length(4)
# JSON chunk: chunkLength(4) + chunkType(4) + chunkData
# BIN chunk:  chunkLength(4) + chunkType(4) + chunkData

MAGIC = b'glTF'
VERSION = 2
JSON_CHUNK_TYPE = 0x4E4F534A  # "JSON"
BIN_CHUNK_TYPE = 0x004E4942   # "BIN\0"

json_chunk = struct.pack('<II', len(json_bytes), JSON_CHUNK_TYPE) + json_bytes
bin_chunk = struct.pack('<II', bin_length, BIN_CHUNK_TYPE) + bin_data

total_length = 12 + len(json_chunk) + len(bin_chunk)
header = MAGIC + struct.pack('<II', VERSION, total_length)

glb = header + json_chunk + bin_chunk

output_path = os.path.join(os.path.dirname(__file__), 'assets', 'panel.glb')
os.makedirs(os.path.dirname(output_path), exist_ok=True)
with open(output_path, 'wb') as f:
    f.write(glb)

print(f"Written {len(glb)} bytes to {output_path}")
print(f"  Vertices: {num_vertices}, Indices: {num_indices} ({num_indices // 3} triangles)")
print(f"  Binary data: {bin_length} bytes")
print(f"  JSON: {len(json_bytes)} bytes")
