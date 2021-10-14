use nalgebra_glm as glm;

pub fn is_point_inside_rectangle(a: glm::Vec2, b: glm::Vec2, c: glm::Vec2, d: glm::Vec2, p: glm::Vec2) -> bool {
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

    let cumulative_area = triangle_area(a, p, d) + triangle_area(d, p, c) + triangle_area(c, p, b) + triangle_area(p, b, a);
    let area = triangle_area(a, b, c) + triangle_area(c, d, a);
    return cumulative_area <= area;
}
