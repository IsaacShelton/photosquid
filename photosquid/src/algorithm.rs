use nalgebra_glm as glm;

pub fn is_point_inside_rectangle(a: glm::Vec2, b: glm::Vec2, c: glm::Vec2, d: glm::Vec2, point: glm::Vec2) -> bool {
    // Returns whether point 'p' is inside the rectangle 'abcd'
    // Where 'a', 'b', 'c', 'd' form edges between each other and the next
    // e.g.
    // A ------------------- B
    // |     P.              |
    // |                     |
    // D ------------------- C
    //
    // The rectangle does not have to be axis-aligned

    fn triangle_area(a: glm::Vec2, b: glm::Vec2, c: glm::Vec2) -> f32 {
        0.5 * ((b.x * a.y - a.x * b.y) + (c.x * b.y - b.x * c.y) + (a.x * c.y - c.x * a.y)).abs()
    }

    let cumulative_area = triangle_area(a, point, d) + triangle_area(d, point, c) + triangle_area(c, point, b) + triangle_area(point, b, a);
    let area = triangle_area(a, b, c) + triangle_area(c, d, a);

    cumulative_area <= area
}

pub fn is_point_inside_triangle(single_point: glm::Vec2, p: [glm::Vec2; 3]) -> bool {
    fn sign(a: glm::Vec2, b: glm::Vec2, c: glm::Vec2) -> f32 {
        (a.x - c.x) * (b.y - c.y) - (b.x - c.x) * (a.y - c.y)
    }

    let d = [(p[0], p[1]), (p[1], p[2]), (p[2], p[0])].map(|(a, b)| sign(single_point, a, b));

    let has_neg = d.iter().any(|distance| *distance < 0.0);
    let has_pos = d.iter().any(|distance| *distance > 0.0);

    !(has_neg && has_pos)
}

pub fn get_distance_between_point_and_triangle(single_point: &glm::Vec2, p: &[glm::Vec2; 3]) -> f32 {
    let ordered = counter_clockwise(*p);
    let [a, b, c] = ordered.each_ref();

    let ab_width = glm::distance(a, b);
    let bc_width = glm::distance(b, c);
    let ca_width = glm::distance(c, a);

    fn get_distance_to_side(point: &glm::Vec2, p1: &glm::Vec2, p2: &glm::Vec2, side_width: f32) -> f32 {
        ((p2.y - p1.y) * point.x - (p2.x - p1.x) * point.y + p2.x * p1.y - p2.y * p1.x) / side_width
    }

    let ab_distance = get_distance_to_side(single_point, a, b, ab_width);
    let bc_distance = get_distance_to_side(single_point, b, c, bc_width);
    let ca_distance = get_distance_to_side(single_point, c, a, ca_width);

    ab_distance.max(bc_distance).max(ca_distance)
}

fn counter_clockwise<'a>(points: [glm::Vec2; 3]) -> [glm::Vec2; 3] {
    use std::cmp::Ordering;

    let mut points = points;
    let center = get_triangle_center(points);

    points.sort_by(|u, v| {
        if is_point_less_in_clockwise(&center, u, v) {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });

    points
}

fn is_point_less_in_clockwise(center: &glm::Vec2, a: &glm::Vec2, b: &glm::Vec2) -> bool {
    if a.x - center.x >= 0.0 && b.x - center.x < 0.0 {
        return true;
    }

    if a.x - center.x < 0.0 && b.x - center.x >= 0.0 {
        return false;
    }

    if a.x - center.x == 0.0 && b.x - center.x == 0.0 {
        if a.y - center.y >= 0.0 || b.y - center.y >= 0.0 {
            return a.y > b.y;
        }
        return b.y > a.y;
    }

    let det: i32 = ((a.x - center.x) * (b.y - center.y) - (b.x - center.x) * (a.y - center.y)) as i32;

    if det != 0 {
        return det < 0;
    }

    let d1: i32 = ((a.x - center.x) * (a.x - center.x) + (a.y - center.y) * (a.y - center.y)) as i32;
    let d2: i32 = ((b.x - center.x) * (b.x - center.x) + (b.y - center.y) * (b.y - center.y)) as i32;
    d1 > d2
}

pub fn get_triangle_center(p: [glm::Vec2; 3]) -> glm::Vec2 {
    p.iter().sum::<glm::Vec2>() / 3.0
}
