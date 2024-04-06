(fabric-library
  (fabric (name "Single") (build (branch (alias Single))))
  (fabric (name "Omni") (build (branch (alias Omni))))
  (fabric (name "Torque") (build (branch (alias Torque))))
  (fabric
    (name "Knee")
    (build
      (branch (alias Omni)
        (face (alias Top:Right) (grow 3))
        (face (alias Front:Left) (grow 3))))
    (shape
      (vulcanize)
      (faces-to-triangles)))
  (fabric
    (name "Flagellum")
    (build
      (branch (alias Single)
        (face (alias :next-base) (grow 20 (scale .9)))))
    (shape
      (vulcanize)
      (faces-to-triangles))
    (pretense (surface :frozen)))
  (fabric
    (name "Tetrapod")
    (build
      (branch (alias Omni)
        (face (alias Top:Right) (grow 3 (scale .9)))
        (face (alias Bottom:Right) (grow 3 (scale .9)))
        (face (alias Back:Left) (grow 3 (scale .9)))
        (face (alias Front:Left) (grow 3 (scale .9)))
        ))
    (shape
      (vulcanize)
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Twist8")
    (build
      (branch (alias Omni)
        (face (alias Top:Right) (grow 12 (scale .92) (mark :ring-a)))
        (face (alias Bottom:Right) (grow 13 (scale .92) (mark :ring-a)))
        (face (alias Back:Left) (grow 12 (scale .92) (mark :ring-b)))
        (face (alias Front:Left) (grow 13 (scale .92) (mark :ring-b)))
        ))
    (shape
      (during 15000 (join :ring-a) (join :ring-b))
      (during 20000 (vulcanize))
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Ring")
    (build
      (branch (alias Single)
        (face (alias :base) (grow 6 (mark :tip)))
        (face (alias :next-base) (grow 5 (mark :tip)))))
    (shape
      (during 40000 (join :tip)))
    (pretense (surface :absent)))
  (fabric
    (name "Halo by Crane")
    (build
      (branch (alias Single) (rotate) (rotate)
        (face (alias :next-base)
          (grow 4 (scale .92)
            (branch (alias Omni)
              (face (alias TopX) (grow 12 (scale .92) (mark :halo-end)))
              (face (alias TopY) (grow 11 (scale .92) (mark :halo-end))))))))
    (shape
      (join :halo-end)
      (during 30000 (remove-spacers)) ; TODO: should automatically happen before vulcanize
      (vulcanize)
      (faces-to-triangles)))
  (fabric
    (name "Pulsating Pavilion")
    (build
      (branch (alias Torque)
        (face (alias Left:Front:Bottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias Far:Base) (grow 8 (scale 0.93) (mark :a))))))
        (face (alias Left:Back:Bottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias Far:Base) (grow 8 (scale 0.93) (mark :b))))))
        (face (alias Right:Front:Bottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias Far:Base) (grow 8 (scale 0.93) (mark :b))))))
        (face (alias Right:Back:Bottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias Far:Base) (grow 8 (scale 0.93)  (mark :a))))))))
    (shape
      (during 35000 (space :a .57) (space :b .57))
      (during 35000 (vulcanize))
      (faces-to-triangles))
    (pretense
      (surface :frozen)
      (muscle 0.2 13000)
      ))
  (fabric
    (name "Tommy Torque")
    (build
      (branch (alias Omni)
        (face (alias Bottom:Left)
          (grow 1
            (branch (alias Torque) (scale .7) (rotate)
              (face (alias Far:Front) (grow 1 (mark :sole))))))
        (face (alias Bottom:Right)
          (grow 1
            (branch (alias Torque) (scale .7)
              (face (alias Far:Front) (grow 1 (mark :sole))))))
        (face (alias Back:Right)
          (branch (alias Torque)
            (face (alias Far:Base)
              (grow 3 (scale .8)))))
        (face (alias Back:Left)
          (branch (alias Torque) (rotate)
            (face (alias Far:Base)
              (grow 3 (scale .8)))))))
    (shape
      (down :sole)
      (faces-to-triangles))
    (pretense
      (surface :frozen)
      (muscle 0.3 12000)
      ))
  (fabric
    (name "Headless Hug")
    (build
      (branch (alias Omni)
        (face (alias Bottom:Left) (grow "....X.." (scale .95) (mark :legs)))
        (face (alias Bottom:Right) (grow "....X.." (scale .95) (mark :legs)))
        (face (alias Top:Left)
          (grow 2 (scale .9)
            (branch (alias Omni)
              (face (alias TopZ) (mark :chest-1))
              (face (alias BotX) (mark :chest-2))
              (face (alias BotY) (grow "....X..." (scale .93) (mark :hands))))))
        (face (alias Top:Right)
          (grow 2 (scale .9)
            (branch (alias Omni)
              (face (alias TopY) (mark :chest-1))
              (face (alias BotZ) (mark :chest-2))
              (face (alias BotX) (grow "....X..." (scale .93) (mark :hands))))))))
    (shape
      (during 15000 (space :legs .3) (space :hands .3) (space :chest-1 .8) (space :chest-2 .2))
      (during 80000 (vulcanize))
      (remove-spacers)
      (faces-to-triangles)))
  (fabric (name "Torque Island")
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
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Triped")
    (build
      (branch (alias Omni)
        (face (alias Bottom:Right) (grow 10 (scale .9) (mark :end)))
        (face (alias Back:Left) (grow 10 (scale .9) (mark :end)))
        (face (alias Front:Left) (grow 10 (scale .9) (mark :end)))
        ))
    (shape
      (during 18000 (space :end 0.4))
      (during 80000 (vulcanize))
      (faces-to-triangles)
      )
    (pretense (surface :bouncy)))
  (fabric
    (name "Convergence")
    (build
      (branch (alias Omni) (seed 1)
        (face (alias Bot) (grow 2 (scale .9)))
        (face (alias TopY) (grow 10 (scale .9) (mark :end)))
        (face (alias TopX) (grow 10 (scale .9) (mark :end)))
        (face (alias TopZ) (grow 10 (scale .9) (mark :end)))
        ))
    (shape
      (during 18000 (join :end (seed 1)))
      (during 80000 (vulcanize))
      (faces-to-triangles)
      )
    (pretense (surface :frozen)))
  )