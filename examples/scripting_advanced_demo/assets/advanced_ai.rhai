// Advanced AI Script
// State variables are initialized in main.rs


fn dist(a, b) {
    let d = a - b;
    return (d.x*d.x + d.y*d.y + d.z*d.z).sqrt();
}

// Perception: Look for player
let player_pos = vec3(5.0, 0.0, 5.0); // Mock player position (in real game, query entity)
let dir = player_pos - position;
let dist_to_player = dist(position, player_pos);
let ray_dir = dir * (1.0 / dist_to_player);
let can_see_player = dist_to_player < 20.0;

// Sync float_pos if it drifted too much (e.g. teleported externally)
if dist(float_pos, position) > 1.5 {
    float_pos = position;
}

// State Machine
switch state {
    "idle" => {
        if can_see_player {
            state = "chase";
            log("Spotted player! Chasing...");
        } else {
            state = "patrol";
        }
    }
    "patrol" => {
        if can_see_player {
            state = "chase";
            log("Spotted player! Chasing...");
        } else {
            let target = patrol_points[patrol_idx];
            if dist(float_pos, target) < 1.0 {
                patrol_idx = (patrol_idx + 1) % patrol_points.len();
                target = patrol_points[patrol_idx];
                path = [];
                current_path_idx = 0;
                log("Patrolling to point " + patrol_idx);
            }
            
            // Follow path
            if path.len() > 0 && current_path_idx < path.len() {
                let next_pt = path[current_path_idx];
                if dist(float_pos, next_pt) < 0.5 {
                    current_path_idx += 1;
                } else {
                    let move_vec = (next_pt - float_pos);
                    // Normalize
                    let len = (move_vec.x*move_vec.x + move_vec.y*move_vec.y + move_vec.z*move_vec.z).sqrt();
                    if len > 0.001 {
                        move_vec = move_vec * (1.0 / len);
                    }
                    
                    float_pos = float_pos + move_vec * (2.0 * delta_time);
                    commands.set_position(entity_id, float_pos);
                }
            } else {
                // No path, just move directly (fallback)
                let move_vec = (target - float_pos);
                 // Normalize
                let len = (move_vec.x*move_vec.x + move_vec.y*move_vec.y + move_vec.z*move_vec.z).sqrt();
                if len > 0.001 {
                    move_vec = move_vec * (1.0 / len);
                }
                float_pos = float_pos + move_vec * (1.0 * delta_time);
                commands.set_position(entity_id, float_pos);
            }
        }
    }
    "chase" => {
        if !can_see_player {
            state = "idle";
            log("Lost player. Idling.");
        } else {
            if dist_to_player < 2.0 {
                state = "attack";
                log("Attacking player!");
            } else {
                // Move towards player
                float_pos = float_pos + ray_dir * (4.0 * delta_time);
                commands.set_position(entity_id, float_pos);
            }
        }
    }
    "attack" => {
        if dist_to_player > 3.0 {
            state = "chase";
        } else {
            commands.apply_damage(target_id, 10.0); // target_id needs to be set
            commands.spawn_particle("hit_effect", player_pos);
        }
    }
}
