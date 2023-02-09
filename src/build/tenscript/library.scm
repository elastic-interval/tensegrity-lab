(library
  (fabric
    (name "Seed")
    (surface :bouncy)
    (build
      (seed :left-mitosis)))
  (fabric
    (name "Knee")
    (build
      (seed :right-omni)
      (branch
        (face F1 (grow 3))
        (face F3 (grow 3))))
    (shape
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Flagellum")
    (build
      (seed :left)
      (grow 20 (scale .9))))
  (fabric
    (name "Halo by Crane")
    (build
      (grow 4 (scale .92)
        (branch
          (face F2 (grow 12 (scale .92) (mark :halo-end)))
          (face F6 (grow 11 (scale .92) (mark :halo-end))))))
    (shape
      (join :halo-end)
      (remove-shapers) ; TODO: should automatically happen before vulcanize
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Headless Hug")
    (build
      (seed :right-omni
        (orient-down F0 F3))
      (branch
        (face F0 (grow "....X.." (scale .95) (mark :legs)))
        (face F3 (grow "....X.." (scale .95) (mark :legs)))
        (face F1
          (grow 2 (scale .9)
            (branch
              (face F5 (mark :shoulders))
              (face F7 (grow "....X..." (scale .93) (mark :hands)))
              )))
        (face F2
          (grow 2 (scale .9)
            (branch
              (face F7 (mark :shoulders))
              (face F5 (grow "....X..." (scale .93) (mark :hands)))
              )))))
    (shape
      (countdown 25000
        (space :legs .5)
        (space :hands .01)
        (space :shoulders .05))
      (countdown 10000 (vulcanize))
      (remove-shapers)
      (replace-faces)))
  (brick :mitosis
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
        (left :top_left :left_back :back_left_top F0)
        (right :top_left :left_front :front_left_top F3)
        (left :bottom_left :left_front :front_left_bottom F1)
        (left :top_right :right_front :front_right_top F2)
        (left :bottom_right :right_back :back_right_bottom F3)
        (right :bottom_left :back_left_bottom :left_back F5)
        (right :top_right :back_right_top :right_back F6)
        (right :bottom_right :front_right_bottom :right_front F7)))))

