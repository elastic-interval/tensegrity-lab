(library
  (fabric
    (name "Seed")
    (build
      (branch (alias Omni)))
    (shape))
  (fabric
    (name "Knee")
    (build
      (branch (alias Omni)
        (face (alias TopX) (grow 3))
        (face (alias TopY) (grow 3))))
    (shape
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Flagellum")
    (build
      (branch (alias Single)
        (face (alias Top) (grow 20 (scale .9)))))
    (shape))
  (fabric
    (name "Halo by Crane")
    (build
      (branch (alias Single)
        (face (alias Top) (grow 4 (scale .92)
          (branch (alias Omni)
            (face (alias TopX) (grow 12 (scale .92) (mark :halo-end)))
            (face (alias TopY) (grow 11 (scale .92) (mark :halo-end))))))
        ))
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
        (right :alpha_z :alpha_y :alpha_x (alias :right :base) (down))
        (right :omega_x :omega_y :omega_z (alias :right Top :next-base))))
    (baked
      (joint -1.4913 1.3144 0.0099)
      (joint 1.4913 1.6921 0.3876)
      (joint 0.0099 0.2107 -0.3876)
      (joint 0.3876 3.1933 -0.0099)
      (joint -0.3876 1.7119 -1.4913)
      (joint -0.0099 2.0895 1.4913)
      (joint -0.6229 1.0791 -0.6229)
      (joint 0.6229 2.3249 0.6229)
      (push 0 1 -0.0531)
      (pull 0 5 0.1171)
      (push 2 3 -0.0531)
      (pull 2 1 0.1170)
      (push 4 5 -0.0531)
      (pull 4 3 0.1171)
      (right 1 3 5 (alias :right Top :next-base))
      (right 4 2 0 (alias :right :base))))
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
        (left :alpha_x :alpha_y :alpha_z (alias :left :base) (down))
        (left :omega_z :omega_y :omega_x (alias :left Top :next-base))))
    (baked
      (joint -1.4913 1.7119 -0.3876)
      (joint 1.4913 2.0895 -0.0099)
      (joint -0.3876 0.2107 0.0099)
      (joint -0.0099 3.1933 0.3876)
      (joint 0.0099 1.3144 -1.4913)
      (joint 0.3876 1.6921 1.4913)
      (joint -0.6229 1.0791 -0.6229)
      (joint 0.6229 2.3249 0.6229)
      (push 4 5 -0.0531)
      (pull 4 1 0.1171)
      (push 0 1 -0.0531)
      (pull 2 5 0.1170)
      (pull 0 3 0.1171)
      (push 2 3 -0.0531)
      (left 0 2 4 (alias :left :base))
      (left 5 3 1 (alias :left Top :next-base))))
  (brick 
    (alias Omni)
    (proto
      (pushes X 3.271 (push :bot_alpha_x :bot_omega_x) (push :top_alpha_x :top_omega_x))
      (pushes Y 3.271 (push :bot_alpha_y :bot_omega_y) (push :top_alpha_y :top_omega_y))
      (pushes Z 3.271 (push :bot_alpha_z :bot_omega_z) (push :top_alpha_z :top_omega_z))
      (faces
        (right :top_omega_x :top_omega_y :top_omega_z (alias :left Top)    (alias :right :base)  (alias Upright Dunno) )
        (left  :top_omega_x :top_alpha_y :bot_omega_z (alias :left TopX)   (alias :right BotX)   (alias Upright Dunno) )
        (left  :top_omega_y :top_alpha_z :bot_omega_x (alias :left TopY)   (alias :right BotY)   (alias Upright Dunno) )
        (left  :top_omega_z :top_alpha_x :bot_omega_y (alias :left TopZ)   (alias :right BotZ)   (alias Upright Dunno) )
        (right :bot_alpha_z :bot_omega_x :top_alpha_y (alias :left BotZ)   (alias :right TopZ)   (alias Upright Dunno) )
        (right :bot_alpha_y :bot_omega_z :top_alpha_x (alias :left BotY)   (alias :right TopY)   (alias Upright Dunno) )
        (right :bot_alpha_x :bot_omega_y :top_alpha_z (alias :left BotX)   (alias :right TopX)   (alias Upright Dunno) )
        (left  :bot_alpha_x :bot_alpha_y :bot_alpha_z (alias :left :base)  (alias :right Top)    (alias Upright Dunno) )
        ))
    (baked
      (joint -1.5556 1.7355 -0.7722)
      (joint 1.5556 1.7355 -0.7722)
      (joint -1.5556 1.7355 0.7722)
      (joint 1.5556 1.7355 0.7722)
      (joint -0.7722 0.1799 -0.0000)
      (joint -0.7722 3.2910 -0.0000)
      (joint 0.7722 0.1799 -0.0000)
      (joint 0.7722 3.2910 -0.0000)
      (joint -0.0000 0.9633 -1.5556)
      (joint -0.0000 0.9633 1.5556)
      (joint -0.0000 2.5076 -1.5556)
      (joint 0.0000 2.5076 1.5556)
      (joint 0.7758 2.5113 0.7758)
      (joint 0.7758 0.9596 0.7758)
      (joint 0.7758 2.5113 -0.7758)
      (joint -0.7758 2.5113 0.7758)
      (joint 0.7758 0.9596 -0.7758)
      (joint -0.7758 0.9596 0.7758)
      (joint -0.7758 2.5113 -0.7758)
      (joint -0.7758 0.9596 -0.7758)
      (push 10 11 -0.0473)
      (push 2 3 -0.0473)
      (push 6 7 -0.0474)
      (push 4 5 -0.0474)
      (push 0 1 -0.0473)
      (push 8 9 -0.0474)
      (right 3 7 11 (alias :left Top) (alias :right :base) (alias Upright Dunno))
      (left 7 10 1 (alias :left TopY) (alias :right BotY) (alias Upright Dunno))
      (left 11 2 5 (alias :left TopZ) (alias :right BotZ) (alias Upright Dunno))
      (right 8 1 6 (alias :left BotZ) (alias :right TopZ) (alias Upright Dunno))
      (right 0 5 10 (alias :left BotX) (alias :right TopX) (alias Upright Dunno))
      (right 4 9 2 (alias :left BotY) (alias :right TopY) (alias Upright Dunno))
      (left 0 4 8 (alias :left :base) (alias :right Top) (alias Upright Dunno))
      (left 3 6 9 (alias :left TopX) (alias :right BotX) (alias Upright Dunno))))
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
        (left  :bottom_left  :left_front  :front_left_bottom  (alias :left :base)      (alias :right Far:Side)  (down))
        (right :bottom_left  :left_back   :back_left_bottom   (alias :left Base:Back)  (alias :right Far:Back)  (down))
        (left  :bottom_right :right_back  :back_right_bottom  (alias :left Far:Back)   (alias :right Base:Back) (down))
        (right :bottom_right :right_front :front_right_bottom (alias :left Far:Side)   (alias :right :base)     (down))
        (left  :top_left     :left_back   :back_left_top      (alias :left Base:Side)  (alias :right Far:Base)        )
        (right :top_left     :left_front  :front_left_top     (alias :left Base:Front) (alias :right Far:Front)       )
        (left  :top_right    :right_front :front_right_top    (alias :left Far:Front)  (alias :right Base:Front)      )
        (right :top_right    :right_back  :back_right_top     (alias :left Far:Base)   (alias :right Base:Side)       )
        ))
    )
  )


