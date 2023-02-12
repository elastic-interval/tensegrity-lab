(library
  (fabric
    (name "Seed")
    (surface :bouncy)
    (build
      (alias Omni)))
  (fabric
    (name "Knee")
    (build
      (alias Omni)
      (branch
        (face (alias TopX) (grow 3))
        (face (alias TopY) (grow 3))))
    (shape
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Flagellum")
    (build
      (alias Right)
      (grow 20 (scale.9))))
  (fabric
    (name "Halo by Crane")
    (build
      (alias Right)
      (grow 4 (scale .92)
        (branch (alias Omni)
          (face (alias TopX) (grow 12 (scale .92) (mark :halo-end)))
          (face (alias TopY) (grow 11 (scale .92) (mark :halo-end))))))
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
    (alias Single)
    (proto
      (pushes X 3.204 (push :alpha_x :omega_x))
      (pushes Y 3.204 (push :alpha_y :omega_y))
      (pushes Z 3.204 (push :alpha_z :omega_z))
      (pulls 2.0
        (pull :alpha_x :omega_z)
        (pull :alpha_y :omega_x)
        (pull :alpha_z :omega_y))
      (faces
        (right :alpha_z :alpha_y :alpha_x (alias Right (base)) (down))
        (right :omega_x :omega_y :omega_z (alias Right (next-base))))))
  (brick ; single-left
    (alias Single)
    (proto
      (pushes X 3.204 (push :alpha_x :omega_x))
      (pushes Y 3.204 (push :alpha_y :omega_y))
      (pushes Z 3.204 (push :alpha_z :omega_z))
      (pulls 2.0
        (pull :alpha_x :omega_y)
        (pull :alpha_y :omega_z)
        (pull :alpha_z :omega_x))
      (faces
        (left :alpha_x :alpha_y :alpha_z (alias Left (base)) (down))
        (left :omega_z :omega_y :omega_x (alias Left (next-base))))))
  (brick 
    (alias Omni)
    (proto
      (pushes X 3.271 (push :bot_alpha_x :bot_omega_x) (push :top_alpha_x :top_omega_x))
      (pushes Y 3.271 (push :bot_alpha_y :bot_omega_y) (push :top_alpha_y :top_omega_y))
      (pushes Z 3.271 (push :bot_alpha_z :bot_omega_z) (push :top_alpha_z :top_omega_z))
      (faces
        (right :top_omega_x :top_omega_y :top_omega_z (alias Left Top)    (alias Right (base)) (alias Upright Dunno) )
        (left  :top_omega_x :top_alpha_y :bot_omega_z (alias Left TopX)   (alias Right BotX)   (alias Upright Dunno) )
        (left  :top_omega_y :top_alpha_z :bot_omega_x (alias Left TopY)   (alias Right BotY)   (alias Upright Dunno) )
        (left  :top_omega_z :top_alpha_x :bot_omega_y (alias Left TopZ)   (alias Right BotZ)   (alias Upright Dunno) )
        (right :bot_alpha_z :bot_omega_x :top_alpha_y (alias Left BotZ)   (alias Right TopZ)   (alias Upright Dunno) )
        (right :bot_alpha_y :bot_omega_z :top_alpha_x (alias Left BotY)   (alias Right TopY)   (alias Upright Dunno) )
        (right :bot_alpha_x :bot_omega_y :top_alpha_z (alias Left BotX)   (alias Right TopX)   (alias Upright Dunno) )
        (left  :bot_alpha_x :bot_alpha_y :bot_alpha_z (alias Left (base)) (alias Right Top)    (alias Upright Dunno) )
        )))
  (brick
    (alias Torque)
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
        (left  :bottom_left  :left_front  :front_left_bottom  (alias Left (base))     (alias Right Far:Side)  (down))
        (right :bottom_left  :left_back   :back_left_bottom   (alias Left Base:Back)  (alias Right Far:Back)  (down))
        (left  :bottom_right :right_back  :back_right_bottom  (alias Left Far:Back)   (alias Right Base:Back) (down))
        (right :bottom_right :right_front :front_right_bottom (alias Left Far:Side)   (alias Right (base))    (down))
        (left  :top_left     :left_back   :back_left_top      (alias Left Base:Side)  (alias Right Far:Base)        )
        (right :top_left     :left_front  :front_left_top     (alias Left Base:Front) (alias Right Far:Front)       )
        (left  :top_right    :right_front :front_right_top    (alias Left Far:Front)  (alias Right Base:Front)      )
        (right :top_right    :right_back  :back_right_top     (alias Left Far:Base)   (alias Right Base:Side)       )
        ))
    )
  )


