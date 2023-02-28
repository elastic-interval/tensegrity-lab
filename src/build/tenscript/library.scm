(library
  (fabric (name "Seed" "Single") (build (branch (alias Single))) (shape))
  (fabric (name "Seed" "Omni") (build (branch (alias Omni))) (shape))
  (fabric (name "Seed" "Torque") (build (branch (alias Torque))) (shape))
  (fabric
    (name "Simple" "Knee")
    (build
      (branch (alias Omni)
        (face (alias Top:Right) (grow 3))
        (face (alias Front:Left) (grow 3))))
    (shape
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Simple" "Flagellum")
    (build
      (branch (alias Single)
        (face (alias :next-base) (grow 20 (scale .9)))))
    (shape
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Art" "Halo by Crane")
    (build
      (branch (alias Single) (rotate) (rotate)
        (face (alias :next-base)
          (grow 4 (scale .92)
             (branch (alias Omni)
               (face (alias TopX) (grow 12 (scale .92) (mark :halo-end)))
               (face (alias TopY) (grow 11 (scale .92) (mark :halo-end))))))))
    (shape
      (join :halo-end)
      (countdown 30000 (remove-shapers)) ; TODO: should automatically happen before vulcanize
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Art" "K-10")
    (build
      (branch (alias Torque)
        (face (alias Left:Front:Bottom)
          (branch (alias Torque)
            (face (alias Far:Front)
              (grow 5))))
        (face (alias Left:Back:Bottom)
          (branch
            (alias Torque)
            (face (alias Far:Front) (grow 5))))
        (face (alias Right:Front:Bottom)
          (grow 2
            (branch (alias Torque)
              (face (alias Far:Front) (grow 1)))))
        (face (alias Right:Back:Bottom)
          (grow 2
            (branch
              (alias Torque)
              (face (alias Far:Front) (grow 1)))))))
    (shape
      (vulcanize)
      (replace-faces)))
    (fabric
      (name "Art" "Headless Hug")
      (build
        (branch (alias Omni)
          (face (alias Bottom:Left) (grow "....X.." (scale .95) (mark :legs)))
          (face (alias Bottom:Right) (grow "....X.." (scale .95) (mark :legs)))
          (face (alias Top:Left)
            (grow 2 (scale .9)
              (branch (alias Omni)
                (face (alias TopZ) (mark :chest-1))
                (face (alias BotX) (mark :chest-2))
                (face (alias BotY) (grow "....X..." (scale .93) (mark :hands)))
                )))
          (face (alias Top:Right)
            (grow 2 (scale .9)
              (branch (alias Omni)
                (face (alias TopY) (mark :chest-1))
                (face (alias BotZ) (mark :chest-2))
                (face (alias BotX) (grow "....X..." (scale .93) (mark :hands)))
                )))))
      (shape
        (countdown 15000
          (space :legs .3)
          (space :hands .3)
          (space :chest-1 .8)
          (space :chest-2 .2)
          )
        (countdown 80000 (vulcanize))
        (remove-shapers)
        (replace-faces)))
  (fabric (name "Art" "Torque Island")
    (build
      (branch (alias Torque)
        (face (alias Left:Front:Bottom)
          (branch (alias Torque) (rotate) (rotate)
            (face (alias Far:Side) (mark :loose))))
        (face (alias Right:Front:Bottom)
          (branch (alias Torque) (rotate) (rotate)
            (face (alias Far:Side)
              (branch (alias Torque)
                (face (alias Far:Side) (mark :loose))))))))
    (shape
      (join :loose)
      (replace-faces)
      (bouncy)))
  (brick ; single-right
    (proto
      (alias Single)
      (pushes X 3.204 (push :alpha_x :omega_x))
      (pushes Y 3.204 (push :alpha_y :omega_y))
      (pushes Z 3.204 (push :alpha_z :omega_z))
      (pulls 2.0
        (pull :alpha_x :omega_z)
        (pull :alpha_y :omega_x)
        (pull :alpha_z :omega_y))
      (faces
        (right :alpha_z :alpha_y :alpha_x (alias :right :base) (alias :seed :base))
        (right :omega_x :omega_y :omega_z (alias :right Top :next-base) (alias :seed :next-base))))
    (baked
      (alias Single)
      (joint -1.4913 1.3143 0.0100)
      (joint 1.4913 1.6920 0.3876)
      (joint 0.0100 0.2107 -0.3876)
      (joint 0.3876 3.1933 -0.0100)
      (joint -0.3876 1.7120 -1.4913)
      (joint -0.0100 2.0896 1.4913)
      (joint -0.6230 1.0791 -0.6229)
      (joint 0.6229 2.3250 0.6229)
      (push 0 1 -0.0531)
      (push 2 3 -0.0531)
      (pull 2 1 0.1171)
      (pull 0 5 0.1171)
      (push 4 5 -0.0531)
      (pull 4 3 0.1171)
      (right 1 3 5 (alias :next-base :right Single Top) (alias :seed Single :next-base))
      (right 4 2 0 (alias :base :right Single) (alias :base :seed Single)))
    )
  (brick ; single-left
    (proto
      (alias Single)
      (pushes X 3.204 (push :alpha_x :omega_x))
      (pushes Y 3.204 (push :alpha_y :omega_y))
      (pushes Z 3.204 (push :alpha_z :omega_z))
      (pulls 2.0
        (pull :alpha_x :omega_y)
        (pull :alpha_y :omega_z)
        (pull :alpha_z :omega_x))
      (faces
        (left :alpha_x :alpha_y :alpha_z (alias :left :base) (alias :seed :base))
        (left :omega_z :omega_y :omega_x (alias :left Top :next-base) (alias :seed :next-base))))
    (baked
      (alias Single)
      (joint -1.4913 1.7120 -0.3876)
      (joint 1.4913 2.0896 -0.0100)
      (joint -0.3876 0.2107 0.0100)
      (joint -0.0100 3.1933 0.3876)
      (joint 0.0100 1.3143 -1.4913)
      (joint 0.3876 1.6920 1.4913)
      (joint -0.6229 1.0791 -0.6230)
      (joint 0.6229 2.3250 0.6229)
      (pull 0 3 0.1171)
      (pull 4 1 0.1171)
      (pull 2 5 0.1171)
      (push 4 5 -0.0531)
      (push 2 3 -0.0531)
      (push 0 1 -0.0531)
      (left 0 2 4 (alias :base :left Single) (alias :base :seed Single))
      (left 5 3 1 (alias :left :next-base Single Top) (alias :seed Single :next-base)))
    )
  (brick
    (proto
      (alias Omni)
      (pushes X 3.271 (push :bot_alpha_x :bot_omega_x) (push :top_alpha_x :top_omega_x))
      (pushes Y 3.271 (push :bot_alpha_y :bot_omega_y) (push :top_alpha_y :top_omega_y))
      (pushes Z 3.271 (push :bot_alpha_z :bot_omega_z) (push :top_alpha_z :top_omega_z))
      (faces
        (right :top_omega_x :top_omega_y :top_omega_z (alias :left Top) (alias :right :base) (alias :seed Top:Right))
        (left :top_omega_x :top_alpha_y :bot_omega_z (alias :left TopX) (alias :right BotX) (alias :seed Front:Right))
        (left :top_omega_y :top_alpha_z :bot_omega_x (alias :left TopY) (alias :right BotY) (alias :seed Back:Right))
        (left :top_omega_z :top_alpha_x :bot_omega_y (alias :left TopZ) (alias :right BotZ) (alias :seed Top:Left))
        (right :bot_alpha_z :bot_omega_x :top_alpha_y (alias :left BotZ) (alias :right TopZ) (alias :seed :base Bottom:Right))
        (right :bot_alpha_y :bot_omega_z :top_alpha_x (alias :left BotY) (alias :right TopY) (alias :seed Front:Left))
        (right :bot_alpha_x :bot_omega_y :top_alpha_z (alias :left BotX) (alias :right TopX) (alias :seed Back:Left))
        (left :bot_alpha_x :bot_alpha_y :bot_alpha_z (alias :left :base) (alias :right Top) (alias :seed :base Bottom:Left))
        ))
    (baked
      (alias Omni)
      (joint -1.5556 1.7355 -0.7722)
      (joint 1.5556 1.7355 -0.7722)
      (joint -1.5556 1.7355 0.7722)
      (joint 1.5556 1.7355 0.7722)
      (joint -0.7723 0.1799 0.0000)
      (joint -0.7722 3.2910 -0.0000)
      (joint 0.7723 0.1799 0.0000)
      (joint 0.7722 3.2910 0.0000)
      (joint 0.0000 0.9632 -1.5556)
      (joint 0.0000 0.9632 1.5556)
      (joint 0.0000 2.5077 -1.5556)
      (joint -0.0000 2.5077 1.5556)
      (joint 0.7758 2.5113 0.7759)
      (joint 0.7759 0.9596 0.7758)
      (joint 0.7758 2.5113 -0.7759)
      (joint -0.7758 2.5113 0.7759)
      (joint 0.7759 0.9596 -0.7758)
      (joint -0.7759 0.9596 0.7758)
      (joint -0.7758 2.5113 -0.7759)
      (joint -0.7759 0.9596 -0.7758)
      (push 6 7 -0.0474)
      (push 2 3 -0.0473)
      (push 0 1 -0.0473)
      (push 10 11 -0.0473)
      (push 4 5 -0.0474)
      (push 8 9 -0.0474)
      (left 7 10 1 (alias :left Omni TopY) (alias :right BotY Omni) (alias :seed Back:Right Omni))
      (right 4 9 2 (alias :left BotY Omni) (alias :right Omni TopY) (alias :seed Front:Left Omni))
      (right 8 1 6 (alias :left BotZ Omni) (alias :right Omni TopZ) (alias :seed :base Bottom:Right Omni))
      (left 3 6 9 (alias :left Omni TopX) (alias :right BotX Omni) (alias :seed Front:Right Omni))
      (right 0 5 10 (alias :left BotX Omni) (alias :right Omni TopX) (alias :seed Back:Left Omni))
      (left 11 2 5 (alias :left Omni TopZ) (alias :right BotZ Omni) (alias :seed Top:Left Omni))
      (left 0 4 8 (alias :base :left Omni) (alias :right Omni Top) (alias :seed :base Bottom:Left Omni))
      (right 3 7 11 (alias :left Omni Top) (alias :base :right Omni) (alias :seed Top:Right Omni)))
    )
  (brick
    (proto
      (alias Torque)
      (pushes X 3.467
        (push :left_front :left_back)
        (push :middle_front :middle_back)
        (push :right_front :right_back))
      (pushes Y 3.467
        (push :front_left_bottom :front_left_top)
        (push :front_right_bottom :front_right_top)
        (push :back_left_bottom :back_left_top)
        (push :back_right_bottom :back_right_top))
      (pushes Z 6.933
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
        (left :bottom_left :left_front :front_left_bottom (alias :left :base) (alias :right Far:Side) (alias :seed Left:Front:Bottom :base))
        (right :bottom_left :left_back :back_left_bottom (alias :left Base:Back) (alias :right Far:Back) (alias :seed Left:Back:Bottom :base))
        (left :bottom_right :right_back :back_right_bottom (alias :left Far:Back) (alias :right Base:Back) (alias :seed Right:Back:Bottom :base))
        (right :bottom_right :right_front :front_right_bottom (alias :left Far:Side) (alias :right :base) (alias :seed Right:Front:Bottom :base))
        (left :top_left :left_back :back_left_top (alias :left Base:Side) (alias :right Far:Base) (alias :seed Left:Back:Top))
        (right :top_left :left_front :front_left_top (alias :left Base:Front) (alias :right Far:Front) (alias :seed Left:Front:Top))
        (left :top_right :right_front :front_right_top (alias :left Far:Front) (alias :right Base:Front) (alias :seed Right:Front:Top))
        (right :top_right :right_back :back_right_top (alias :left Far:Base) (alias :right Base:Side) (alias :seed Right:Back:Top))
        )
      )
    (baked
      (alias Torque)
      (joint -1.6518 1.8335 -2.6932)
      (joint 1.6518 1.8335 -2.6932)
      (joint -1.6916 1.8335 0.0000)
      (joint 1.6916 1.8335 0.0000)
      (joint -1.6518 1.8335 2.6932)
      (joint 1.6518 1.8335 2.6932)
      (joint -1.0938 0.1732 -2.0302)
      (joint -1.0936 3.4935 -2.0303)
      (joint -1.0938 0.1732 2.0302)
      (joint -1.0936 3.4935 2.0303)
      (joint 1.0938 0.1732 -2.0302)
      (joint 1.0936 3.4935 -2.0303)
      (joint 1.0938 0.1732 2.0302)
      (joint 1.0936 3.4935 2.0303)
      (joint 0.0000 2.6390 -3.3251)
      (joint 0.0000 2.6390 3.3251)
      (joint 0.0000 1.0279 -3.3251)
      (joint 0.0000 1.0279 3.3251)
      (joint -0.9063 1.0235 -2.6963)
      (joint 0.9063 1.0235 -2.6963)
      (joint 0.9063 1.0235 2.6963)
      (joint -0.9063 1.0235 2.6963)
      (joint 0.9063 2.6434 -2.6963)
      (joint -0.9063 2.6434 -2.6963)
      (joint -0.9063 2.6434 2.6963)
      (joint 0.9063 2.6434 2.6963)
      (pull 3 13 0.0772)
      (pull 2 9 0.0772)
      (push 10 11 -0.0408)
      (pull 2 6 0.0772)
      (pull 3 10 0.0772)
      (push 14 15 -0.0393)
      (pull 2 8 0.0772)
      (push 12 13 -0.0408)
      (pull 3 11 0.0772)
      (pull 3 12 0.0772)
      (push 2 3 -0.0229)
      (pull 2 7 0.0772)
      (push 16 17 -0.0393)
      (push 6 7 -0.0408)
      (push 4 5 -0.0456)
      (push 8 9 -0.0408)
      (push 0 1 -0.0456)
      (right 17 4 8 (alias :left Far:Side Torque) (alias :base :right Torque) (alias :base :seed Right:Front:Bottom Torque))
      (left 14 1 11 (alias :left Base:Side Torque) (alias :right Far:Base Torque) (alias :seed Left:Back:Top Torque))
      (right 14 0 7 (alias :left Base:Front Torque) (alias :right Far:Front Torque) (alias :seed Left:Front:Top Torque))
      (left 17 5 12 (alias :left Far:Back Torque) (alias :right Base:Back Torque) (alias :base :seed Right:Back:Bottom Torque))
      (right 15 5 13 (alias :left Far:Base Torque) (alias :right Base:Side Torque) (alias :seed Right:Back:Top Torque))
      (left 15 4 9 (alias :left Far:Front Torque) (alias :right Base:Front Torque) (alias :seed Right:Front:Top Torque))
      (left 16 0 6 (alias :base :left Torque) (alias :right Far:Side Torque) (alias :base :seed Left:Front:Bottom Torque))
      (right 16 1 10 (alias :left Base:Back Torque) (alias :right Far:Back Torque) (alias :base :seed Left:Back:Bottom Torque)))
    )
  )


