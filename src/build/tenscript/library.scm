(library
  (fabric
    (name "Seed")
    (surface :bouncy)
    (build
      (alias Omni::Left::Bot)))
  (fabric
    (name "Knee")
    (build
      (alias Omni::Right::Bot)
      (branch
        (face (alias Omni::Right::TopX) (grow 3))
        (face (alias Omni::Right::TopY) (grow 3))))
    (shape
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Flagellum")
    (build
      (alias Right::Bot)
      (grow 20 (scale.9))))
  (fabric
    (name "Halo by Crane")
    (build
      (alias Right::Bot)
      (grow 4 (scale.92)
        (branch
          (face (alias Omni::Left::TopX) (grow 2 (scale.92) (mark :halo-end)))
          (face (alias Omni::Left::TopY) (grow 11 (scale.92) (mark :halo-end))))))
    (shape
      (join :halo-end)
      (remove-shapers) ; TODO: should automatically happen before vulcanize
      (vulcanize)
      (replace-faces)))
;  (fabric
;    (name "Headless Hug")
;    (build
;      (seed :right-omni
;        (orient-down F0 F3))
;      (branch
;        (face F0 (grow "....X.." (scale.95) (mark :legs)))
;        (face F3 (grow "....X.." (scale.95) (mark :legs)))
;        (face F1
;          (grow 2 (scale.9)
;            (branch
;              (face F5 (mark :shoulders))
;              (face F7 (grow "....X..." (scale.93) (mark :hands)))
;              )))
;        (face F2
;          (grow 2 (scale.9)
;            (branch
;              (face F7 (mark :shoulders))
;              (face F5 (grow "....X..." (scale.93) (mark :hands)))
;              )))))
;    (shape
;      (countdown 25000
;        (space :legs.5)
;        (space :hands.01)
;        (space :shoulders.05))
;      (countdown 10000 (vulcanize))
;      (remove-shapers)
;      (replace-faces)))
  (brick ; single-right
    (proto
      (pushes X 3.204 (push :alpha_x :omega_x))
      (pushes Y 3.204 (push :alpha_y :omega_y))
      (pushes Z 3.204 (push :alpha_z :omega_z))
      (pulls 2.0
        (pull :alpha_x :omega_z)
        (pull :alpha_y :omega_x)
        (pull :alpha_z :omega_y))
      (faces
        (right :alpha_z :alpha_y :alpha_x (alias Right::Bot))
        (right :omega_x :omega_y :omega_z (alias Right::Top))))
    (baked
      (joint -0.5000 0.0000 -0.8660)
      (joint 0.0068 1.9619 1.0000)
      (joint -0.5000 0.0000 0.8660)
      (joint 0.8626 1.9619 -0.5059)
      (joint 1.0000 0.0000 0.0000)
      (joint -0.8694 1.9619 -0.4941)
      (joint -0.0000 0.0001 -0.0000)
      (joint -0.0000 1.9618 0.0000)
      (pull 2 1 0.1171)
      (push 4 5 -0.0531)
      (push 0 1 -0.0531)
      (pull 0 5 0.1171)
      (pull 4 3 0.1171)
      (push 2 3 -0.0531)
      (right 4 2 0 (alias Right::Bot))
      (right 1 3 5 (alias Right::Top))))
  (brick ; single-left
    (proto
      (pushes X 3.204 (push :alpha_x :omega_x))
      (pushes Y 3.204 (push :alpha_y :omega_y))
      (pushes Z 3.204 (push :alpha_z :omega_z))
      (pulls 2.0
        (pull :alpha_x :omega_y)
        (pull :alpha_y :omega_z)
        (pull :alpha_z :omega_x))
      (faces
        (left :alpha_x :alpha_y :alpha_z (alias Left::Bot))
        (left :omega_z :omega_y :omega_x (alias Left::Top))))
    (baked
      (joint 1.0000 -0.0000 -0.0000)
      (joint -0.8694 1.9619 0.4941)
      (joint -0.5000 -0.0000 -0.8660)
      (joint 0.8626 1.9619 0.5059)
      (joint -0.5000 0.0000 0.8660)
      (joint 0.0068 1.9619 -1.0000)
      (joint 0.0000 0.0001 -0.0000)
      (joint 0.0000 1.9618 -0.0000)
      (push 0 1 -0.0531)
      (pull 0 3 0.1171)
      (push 4 5 -0.0531)
      (pull 2 5 0.1171)
      (push 2 3 -0.0531)
      (pull 4 1 0.1171)
      (left 5 3 1 (alias Left::Top))
      (left 0 2 4 (alias Left::Bot))))
  (brick ; omni
    (proto
      (pushes X 3.271 (push :bot_alpha_x :bot_omega_x) (push :top_alpha_x :top_omega_x))
      (pushes Y 3.271 (push :bot_alpha_y :bot_omega_y) (push :top_alpha_y :top_omega_y))
      (pushes Z 3.271 (push :bot_alpha_z :bot_omega_z) (push :top_alpha_z :top_omega_z))
      (faces
        (right :top_omega_x :top_omega_y :top_omega_z (alias Omni::Left::Top) (alias Omni::Right::Bot))
        (left :top_omega_x :top_alpha_y :bot_omega_z (alias Omni::Left::TopX) (alias Omni::Right::BotX))
        (left :top_omega_y :top_alpha_z :bot_omega_x (alias Omni::Left::TopY) (alias Omni::Right::BotY))
        (left :top_omega_z :top_alpha_x :bot_omega_y (alias Omni::Left::TopZ) (alias Omni::Right::BotZ))
        (right :bot_alpha_z :bot_omega_x :top_alpha_y (alias Omni::Left::BotZ) (alias Omni::Right::TopZ))
        (right :bot_alpha_y :bot_omega_z :top_alpha_x (alias Omni::Left::BotY) (alias Omni::Right::TopY))
        (right :bot_alpha_x :bot_omega_y :top_alpha_z (alias Omni::Left::BotX) (alias Omni::Right::TopX))
        (left :bot_alpha_x :bot_alpha_y :bot_alpha_z (alias Omni::Left::Bot) (alias Omni::Right::Top)))))
  (brick ; mitosis
    (proto
      (pushes X 3.467
        (push :left_front :left_back)
        (push :middle_front :middle_back)
        (push :right_front :right_back))
      (pushes Y 3.467
        (push :front_left_bottom :front_left_top)
        (push :front_right_bottom :front_right_top)
        (push :back_left_bottom :back_left_top)
        (push :back_right_bottom :back_right_top))
      (pushes Z 6.934
        (push :top_left :top_right)
        (push :bottom_left :bottom_right))
      (pulls 2.5
        (pull :middle_front :front_left_bottom)
        (pull :middle_front :front_left_top)
        (pull :middle_front :front_right_bottom)
        (pull :middle_front :front_right_top)
        (pull :middle_back :back_left_bottom)
        (pull :middle_back :back_left_top)
        (pull :middle_back :back_right_bottom)
        (pull :middle_back :back_right_top))
      (faces
        (left :top_left :left_back :back_left_top (alias Top))
        (right :top_left :left_front :front_left_top (alias TopX))
        (left :bottom_left :left_front :front_left_bottom (alias TopY))
        (left :top_right :right_front :front_right_top (alias TopZ))
        (left :bottom_right :right_back :back_right_bottom (alias BotX))
        (right :bottom_left :left_back :back_left_bottom (alias BotZ))
        (right :top_right :right_back :back_right_top (alias BotY))
        (right :bottom_right :right_front :front_right_bottom (alias Bot))))
    (baked
      (joint -1.5556 -0.0000 -0.7722)
      (joint 1.5556 0.0000 -0.7722)
      (joint -1.5556 0.0000 0.7722)
      (joint 1.5556 -0.0000 0.7722)
      (joint -0.7722 -1.5556 -0.0000)
      (joint -0.7722 1.5556 0.0000)
      (joint 0.7722 -1.5556 -0.0000)
      (joint 0.7722 1.5556 0.0000)
      (joint -0.0000 -0.7722 -1.5556)
      (joint 0.0000 -0.7722 1.5556)
      (joint -0.0000 0.7722 -1.5556)
      (joint 0.0000 0.7722 1.5556)
      (joint 0.7758 0.7758 0.7758)
      (joint 0.7758 -0.7758 0.7758)
      (joint 0.7758 0.7758 -0.7758)
      (joint -0.7758 0.7758 0.7758)
      (joint 0.7758 -0.7758 -0.7758)
      (joint -0.7758 -0.7758 0.7758)
      (joint -0.7758 0.7758 -0.7758)
      (joint -0.7758 -0.7758 -0.7758)
      (push 8 9 -0.0473)
      (push 0 1 -0.0473)
      (push 4 5 -0.0473)
      (push 6 7 -0.0473)
      (push 2 3 -0.0473)
      (push 10 11 -0.0473)
      (right 8 1 6 (alias Omni::Left::BotZ) (alias Omni::Right::TopZ))
      (left 3 6 9 (alias Omni::Left::TopX) (alias Omni::Right::BotX))
      (right 0 5 10 (alias Omni::Left::BotX) (alias Omni::Right::TopX))
      (left 0 4 8 (alias Omni::Left::Bot) (alias Omni::Right::Top))
      (left 7 10 1 (alias Omni::Left::TopY) (alias Omni::Right::BotY))
      (left 11 2 5 (alias Omni::Left::TopZ) (alias Omni::Right::BotZ))
      (right 4 9 2 (alias Omni::Left::BotY) (alias Omni::Right::TopY))
      (right 3 7 11 (alias Omni::Left::Top) (alias Omni::Right::Bot))))
  )


