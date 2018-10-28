use cgmath::{vec2, vec3, Matrix4, Vector2, Vector3};
use direction::{CardinalDirection, OrdinalDirection, OrdinalDirections};
use grid_2d::{Coord, Grid, Size};

#[derive(Debug, Clone, Copy)]
enum CellType {
    Wall,
    Floor,
}

// Inner:
// #.
// ##
//
// Outer:
// ..
// #.
//
// Left:
// ..
// ##
//
// Right:
// #.
// #.

struct Config {
    cell_size_px: f32,
}

struct Style {
    width_px: f32,
    height_px: f32,
    face_tex_top_left_px: Vector2<f32>,
    top_tex_top_left_px: Vector2<f32>,
}

struct BaseAttribute {
    face_tex_offset_px_x: f32,
    space_coord_px: Vector2<f32>,
}

struct TopAttribute {
    tex_offset_px: Vector2<f32>,
    space_coord_px: Vector2<f32>,
}

impl TopAttribute {
    fn new(piece_tex_offset_px: Vector2<f32>, space_coord_px: Vector2<f32>) -> Self {
        let tex_offset_px =
            piece_tex_offset_px + vec2(space_coord_px.x, -space_coord_px.y);
        Self {
            tex_offset_px,
            space_coord_px,
        }
    }
}

#[derive(Debug, Clone)]
struct Attribute {
    space_coord_px: Vector3<f32>,
    tex_coord_px: Vector2<f32>,
}

#[derive(Debug)]
struct RelativeBuffers {
    attributes: Vec<Attribute>,
    indices: Vec<u32>,
}

impl RelativeBuffers {
    fn concat(&self, b: &Self) -> Self {
        let attributes = self
            .attributes
            .iter()
            .chain(b.attributes.iter())
            .cloned()
            .collect::<Vec<_>>();
        let indices = self
            .indices
            .iter()
            .cloned()
            .chain(b.indices.iter().map(|i| i + self.attributes.len() as u32))
            .collect::<Vec<_>>();
        Self {
            attributes,
            indices,
        }
    }
    fn concat_all<I>(i: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        let mut attributes = Vec::new();
        let mut indices = Vec::new();

        for b in i {
            indices.extend(b.indices.iter().map(|&i| i + attributes.len() as u32));
            attributes.extend_from_slice(&b.attributes);
        }

        Self {
            attributes,
            indices,
        }
    }
    fn transform(self, m: Matrix4<f32>) -> Self {
        let Self {
            mut attributes,
            indices,
        } = self;
        attributes.iter_mut().for_each(|a| {
            a.space_coord_px = (m * a.space_coord_px.extend(1.)).truncate();
        });
        Self {
            attributes,
            indices,
        }
    }
}

const BASE_TOP_ALTERNATING_INDICES_1: &[u32] = &[0, 1, 2, 1, 3, 2];
const BASE_TOP_ALTERNATING_INDICES_2: &[u32] = &[0, 1, 2, 1, 3, 2, 2, 3, 4, 3, 5, 4];

fn make_edge_base(
    piece: Piece,
    style: &Style,
    config: &Config,
) -> (Vec<BaseAttribute>, &'static [u32]) {
    let s = config.cell_size_px;
    let w = style.width_px;
    match piece {
        Piece::Inner => (
            vec![
                BaseAttribute {
                    face_tex_offset_px_x: 0.,
                    space_coord_px: vec2(s, w),
                },
                BaseAttribute {
                    // XXX this will produce artifacts where an inner or outer
                    // edge meets another piece of wall unless s == w * 2
                    face_tex_offset_px_x: s - w,
                    space_coord_px: vec2(w, w),
                },
                BaseAttribute {
                    face_tex_offset_px_x: 2. * (s - w),
                    space_coord_px: vec2(w, s),
                },
            ],
            BASE_TOP_ALTERNATING_INDICES_2,
        ),
        Piece::Outer => (
            vec![
                BaseAttribute {
                    face_tex_offset_px_x: 0.,
                    space_coord_px: vec2(0., w),
                },
                BaseAttribute {
                    face_tex_offset_px_x: w,
                    space_coord_px: vec2(w, w),
                },
                BaseAttribute {
                    face_tex_offset_px_x: 2. * w,
                    space_coord_px: vec2(0., w),
                },
            ],
            BASE_TOP_ALTERNATING_INDICES_2,
        ),
        Piece::Left => (
            vec![
                BaseAttribute {
                    face_tex_offset_px_x: 0.,
                    space_coord_px: vec2(s, w),
                },
                BaseAttribute {
                    face_tex_offset_px_x: s,
                    space_coord_px: vec2(0., w),
                },
            ],
            BASE_TOP_ALTERNATING_INDICES_1,
        ),
        Piece::Right => (
            vec![
                BaseAttribute {
                    face_tex_offset_px_x: 0.,
                    space_coord_px: vec2(w, 0.),
                },
                BaseAttribute {
                    face_tex_offset_px_x: s,
                    space_coord_px: vec2(w, s),
                },
            ],
            BASE_TOP_ALTERNATING_INDICES_1,
        ),
    }
}

fn make_faces(piece: Piece, style: &Style, config: &Config) -> RelativeBuffers {
    let (edge_base, indices) = make_edge_base(piece, style, config);
    let base = edge_base.iter().map(|a| {
        let tex_coord_px =
            vec2(a.face_tex_offset_px_x, style.height_px) + style.face_tex_top_left_px;
        let space_coord_px = vec3(a.space_coord_px.x, 0., a.space_coord_px.y);
        Attribute {
            tex_coord_px,
            space_coord_px,
        }
    });
    let top = edge_base.iter().map(|a| {
        let tex_coord_px = vec2(a.face_tex_offset_px_x, 0.) + style.face_tex_top_left_px;
        let space_coord_px =
            vec3(a.space_coord_px.x, style.height_px, a.space_coord_px.y);
        Attribute {
            tex_coord_px,
            space_coord_px,
        }
    });
    let base_top_alternating = base
        .zip(top)
        .flat_map(|(base, top)| vec![base, top])
        .collect::<Vec<_>>();

    RelativeBuffers {
        attributes: base_top_alternating,
        indices: indices.iter().cloned().collect(),
    }
}

fn make_rect_top(
    size: Vector2<f32>,
    piece_tex_offset_px: Vector2<f32>,
) -> (Vec<TopAttribute>, &'static [u32]) {
    const INDICES: &[u32] = &[0, 1, 2, 0, 2, 3];
    let attributes = vec![
        TopAttribute::new(piece_tex_offset_px, vec2(0., 0.)),
        TopAttribute::new(piece_tex_offset_px, vec2(0., size.y)),
        TopAttribute::new(piece_tex_offset_px, size),
        TopAttribute::new(piece_tex_offset_px, vec2(size.x, 0.)),
    ];
    (attributes, INDICES)
}

fn make_top(piece: Piece, style: &Style, config: &Config) -> RelativeBuffers {
    let s = config.cell_size_px;
    let w = style.width_px;
    let (attributes, indices) = match piece {
        Piece::Inner => {
            const INDICES: &[u32] = &[0, 1, 2, 0, 2, 3, 2, 4, 3, 2, 5, 4];
            let piece_tex_offset_px = vec2(s, s);
            (
                vec![
                    TopAttribute::new(piece_tex_offset_px, vec2(0., s)),
                    TopAttribute::new(piece_tex_offset_px, vec2(w, s)),
                    TopAttribute::new(piece_tex_offset_px, vec2(w, w)),
                    TopAttribute::new(piece_tex_offset_px, vec2(0., 0.)),
                    TopAttribute::new(piece_tex_offset_px, vec2(s, 0.)),
                    TopAttribute::new(piece_tex_offset_px, vec2(s, w)),
                ],
                INDICES,
            )
        }
        Piece::Outer => make_rect_top(vec2(w, w), vec2(0., 2. * s)),
        Piece::Left => make_rect_top(vec2(s, w), vec2(s, 2. * s)),
        Piece::Right => make_rect_top(vec2(w, s), vec2(0., s)),
    };
    let attributes = attributes
        .iter()
        .map(|a| {
            let space_coord_px =
                vec3(a.space_coord_px.x, style.height_px, a.space_coord_px.y);
            let tex_coord_px = a.tex_offset_px + style.top_tex_top_left_px;
            Attribute {
                space_coord_px,
                tex_coord_px,
            }
        })
        .collect::<Vec<_>>();
    RelativeBuffers {
        attributes,
        indices: indices.iter().cloned().collect(),
    }
}

fn make_geometry(piece: Piece, style: &Style, config: &Config) -> RelativeBuffers {
    let top = make_top(piece, style, config);
    let faces = make_faces(piece, style, config);
    top.concat(&faces)
}

fn move_to_cell_centre(coord: Coord, config: &Config) -> Matrix4<f32> {
    let position = vec2(
        coord.x as f32 * config.cell_size_px,
        coord.y as f32 * config.cell_size_px,
    ) + vec2(config.cell_size_px / 2., config.cell_size_px / 2.);
    Matrix4::from_translation(vec3(position.x, 0., position.y))
}

fn rotate_to_direction(direction: OrdinalDirection) -> Matrix4<f32> {
    let angle = match direction {
        OrdinalDirection::NorthEast => 0.,
        OrdinalDirection::SouthEast => ::std::f32::consts::PI / 2.,
        OrdinalDirection::SouthWest => ::std::f32::consts::PI,
        OrdinalDirection::NorthWest => -::std::f32::consts::PI / 2.,
    };
    Matrix4::from_angle_y(cgmath::Rad(angle))
}

#[derive(Debug, Clone, Copy)]
enum Piece {
    Inner,
    Outer,
    Left,
    Right,
}

impl Piece {
    fn choose(
        neigh_a: (CellType, CardinalDirection),
        neigh_b: (CellType, CardinalDirection),
    ) -> Self {
        let (wall_direction, floor_direction) = match (neigh_a.0, neigh_b.0) {
            (CellType::Floor, CellType::Floor) => return Piece::Outer,
            (CellType::Wall, CellType::Wall) => return Piece::Inner,
            (CellType::Wall, CellType::Floor) => (neigh_a.1, neigh_b.1),
            (CellType::Floor, CellType::Wall) => (neigh_b.1, neigh_a.1),
        };

        if wall_direction.right90() == floor_direction {
            return Piece::Right;
        }

        if wall_direction.left90() == floor_direction {
            return Piece::Left;
        }

        unreachable!()
    }
}

#[derive(Debug, Clone, Copy)]
struct Quarter {
    piece: Piece,
}

impl Quarter {
    fn from_grid(
        grid: &Grid<CellType>,
        coord: Coord,
        direction: OrdinalDirection,
    ) -> Self {
        let (card_a, card_b) = direction.to_cardinals();
        let cell_type_a = grid
            .get(coord + card_a.coord())
            .cloned()
            .unwrap_or(CellType::Floor);
        let cell_type_b = grid
            .get(coord + card_b.coord())
            .cloned()
            .unwrap_or(CellType::Floor);
        let piece = Piece::choose((cell_type_a, card_a), (cell_type_b, card_b));
        Self { piece }
    }
}

#[derive(Debug)]
struct CellDetails {
    quarters: [Quarter; 4],
}

impl CellDetails {
    fn outer() -> Self {
        let quarter = Quarter {
            piece: Piece::Outer,
        };
        Self {
            quarters: [quarter, quarter, quarter, quarter],
        }
    }
    fn from_grid(grid: &Grid<CellType>, coord: Coord) -> Option<Self> {
        if let CellType::Floor = grid.get(coord).cloned().unwrap_or(CellType::Floor) {
            return None;
        }
        let mut cell_details = Self::outer();
        for o in OrdinalDirections {
            cell_details.quarters[o as usize] = Quarter::from_grid(grid, coord, o);
        }
        Some(cell_details)
    }
    fn make_geometry(
        &self,
        coord: Coord,
        style: &Style,
        config: &Config,
    ) -> Vec<RelativeBuffers> {
        let translate = move_to_cell_centre(coord, config);
        OrdinalDirections
            .into_iter()
            .zip(self.quarters.iter())
            .map(|(o, q)| {
                let rotate = rotate_to_direction(o);
                make_geometry(q.piece, style, config).transform(translate * rotate)
            })
            .collect()
    }
}

fn main() {
    let terrain_vecs = include_str!("terrain_strings.txt")
        .split("\n")
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let width = terrain_vecs[0].len();
    let height = terrain_vecs.len();
    assert!(
        terrain_vecs.iter().all(|v| v.len() == width),
        "inconsistent width"
    );
    let size = Size::new(width as u32, height as u32);
    let type_grid = Grid::new_from_fn(size, |coord| {
        match terrain_vecs[coord.y as usize][coord.x as usize] {
            '.' => CellType::Floor,
            '#' => CellType::Wall,
            _ => panic!("unknown char"),
        }
    });
    let detail_grid =
        Grid::new_from_fn(size, |coord| CellDetails::from_grid(&type_grid, coord));

    let style = Style {
        width_px: 8.,
        height_px: 32.,
        face_tex_top_left_px: vec2(32., 0.),
        top_tex_top_left_px: vec2(0., 0.),
    };

    let config = Config { cell_size_px: 16. };

    let geometry_iter = detail_grid
        .enumerate()
        .filter_map(|(coord, cell)| cell.as_ref().map(|cell| (coord, cell)))
        .flat_map(|(coord, cell)| cell.make_geometry(coord, &style, &config));

    let geometry = RelativeBuffers::concat_all(geometry_iter);

    println!("{:#?}", geometry.attributes);
}
