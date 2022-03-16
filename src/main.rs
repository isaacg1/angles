#![feature(array_zip)]
use std::collections::HashMap;

use image::{ImageBuffer, RgbImage};
use rand::prelude::*;

use std::f64::{consts::PI, INFINITY};

type Color = [u8; 3];
type ColorBase = [u8; 3];

fn color_base_to_color(cb: ColorBase, color_size: u64) -> Color {
    cb.map(|cbc| (cbc as u64 * 255 / (color_size - 1)) as u8)
}
type ColorOffset = [i16; 3];
type Location = [usize; 2];
type LocationOffset = [isize; 2];

#[derive(Debug, Clone, Copy)]
struct Pixel {
    color: Color,
    direction: f64,
}

fn make_image(scale: u64, spread: f64, num_seeds: usize, seed: u64) -> RgbImage {
    let mut rng = StdRng::seed_from_u64(seed);
    let size = scale.pow(3) as usize;
    let color_size = scale.pow(2);
    let mut color_bases: Vec<ColorBase> = (0..scale.pow(6))
        .map(|n| {
            let r_base = n % color_size;
            let g_base = (n / color_size) % color_size;
            let b_base = n / color_size.pow(2);
            [r_base as u8, g_base as u8, b_base as u8]
        })
        .collect();
    let mut color_offsets: Vec<ColorOffset> = color_bases
        .iter()
        .map(|color| color.map(|c| c as i16))
        .flat_map(|color| {
            vec![
                [color[0], color[1], color[2]],
                [color[0], color[1], -color[2]],
                [color[0], -color[1], color[2]],
                [color[0], -color[1], -color[2]],
                [-color[0], color[1], color[2]],
                [-color[0], color[1], -color[2]],
                [-color[0], -color[1], color[2]],
                [-color[0], -color[1], -color[2]],
            ]
            .into_iter()
        })
        .collect();
    color_bases.shuffle(&mut rng);
    color_offsets
        .sort_by_key(|color_offset| color_offset.map(|c| (c as i64).pow(2)).iter().sum::<i64>());
    let mut location_offsets: Vec<LocationOffset> = (0..scale.pow(6) / 2)
        .flat_map(|n| {
            let i = (n as usize % size) as isize;
            let j = (n as usize / size) as isize;
            vec![[i, j], [i, -j], [-i, j], [-i, -j]].into_iter()
        })
        .collect();
    location_offsets
        .sort_by_key(|location_offset| location_offset.map(|l| l.pow(2)).iter().sum::<isize>());
    let mut grid: Vec<Vec<Option<Pixel>>> = vec![vec![None; size]; size];
    let mut color_base_to_location: HashMap<ColorBase, Location> = HashMap::new();
    let mut seed_locs: Vec<Location> = vec![];
    for (i, color_base) in color_bases.into_iter().enumerate() {
        if i < num_seeds {
            let mut row = rng.gen_range(0..size);
            let mut col = rng.gen_range(0..size);
            loop {
                let mut too_close = false;
                for loc in &seed_locs {
                    let dist_sq: isize = loc.zip([row, col]).map(|(l1, l2)| {
                        let il1 = l1 as isize;
                        let il2 = l2 as isize;
                        (il1 - il2)
                            .abs()
                            .min(il1 - il2 + size as isize)
                            .min(il1 - il2 + size as isize)
                    }).map(|d| d.pow(2)).iter().sum::<isize>();
                    let dist: f64 = (dist_sq as f64).sqrt();
                    let min_spacing = size as f64 / (2.0 * (num_seeds as f64).sqrt());
                    if dist < min_spacing {
                        too_close = true;
                    }
                }
                if !too_close {
                    break
                }
                row = rng.gen_range(0..size);
                col = rng.gen_range(0..size);
            }
            let angle = rng.gen_range(0.0..2.0 * PI);
            let pixel = Pixel {
                color: color_base_to_color(color_base, color_size),
                direction: angle,
            };
            grid[row][col] = Some(pixel);
            color_base_to_location.insert(color_base, [row, col]);
            seed_locs.push([row, col]);
            continue;
        }
        let most_similar_location: Location = color_offsets
            .iter()
            .filter_map(|color_offset| {
                let prov_new_color_base = color_base.zip(*color_offset).map(|(c, co)| c as i16 + co);
                if prov_new_color_base.iter().any(|&c| c < 0 || c > 255) {
                    None
                } else {
                    let new_color_base = prov_new_color_base.map(|c| c as u8);
                    color_base_to_location.get(&new_color_base).copied()
                }
            })
            .next()
            .unwrap();
        let direction = grid[most_similar_location[0]][most_similar_location[1]]
            .expect("Most similar present")
            .direction;
        let mut closest_location = None;
        let mut closest_angle = None;
        let mut closest_distance = INFINITY;
        // TODO: Some kind of early exit?
        for &location_offset in &location_offsets {
            let prov_location = most_similar_location
                .zip(location_offset)
                .map(|(l, lo)| ((l as isize + lo) + size as isize) % size as isize);
            let location = prov_location.map(|l| l as usize);
            if grid[location[0]][location[1]].is_none() {
                let angle = (location_offset[0] as f64).atan2(location_offset[1] as f64);
                let mut diff = angle - direction;
                if diff < -PI {
                    diff += 2.0 * PI
                } else if diff > PI {
                    diff -= 2.0 * PI
                }
                if diff.abs() < spread {
                    closest_location = Some(location);
                    closest_angle = Some(angle);
                    break;
                } else {
                    let gap_angle = diff.abs() - spread;
                    let total_squared_distance: isize =
                        location_offset.map(|lo| lo.pow(2)).iter().sum();
                    let total_distance = (total_squared_distance as f64).sqrt();
                    let scaled_distance = total_distance * gap_angle;
                    if closest_distance > scaled_distance {
                        closest_distance = scaled_distance;
                        closest_location = Some(location);
                        closest_angle = Some(angle);
                    }
                }
            }
        }
        let closest_location = closest_location.expect("Found a slot");
        let direction = closest_angle.unwrap();
        let pixel = Pixel { color: color_base_to_color(color_base, color_size), direction };
        grid[closest_location[0]][closest_location[1]] = Some(pixel);
        color_base_to_location.insert(color_base, closest_location);
    }
    let mut img: RgbImage = ImageBuffer::new(size as u32, size as u32);
    for (i, row) in grid.into_iter().enumerate() {
        for (j, pixel) in row.into_iter().enumerate() {
            if let Some(pixel) = pixel {
                img.put_pixel(i as u32, j as u32, image::Rgb(pixel.color))
            }
        }
    }
    img
}

fn main() {
    for scale in 7..=10 {
        let spread = 0.15;
        let num_seeds = 20;
        let seed = 0;
        let filename = format!("img-{}-{}-{}-{}.png", scale, spread, num_seeds, seed);
        println!("Start {}", filename);
        let img = make_image(scale, spread, num_seeds, seed);
        img.save(&filename).unwrap();
    }
}
