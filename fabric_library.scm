(fabric-library
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
      (vulcanize)
      (faces-to-triangles))
    (pretense (surface :frozen)))
  (fabric
    (name "Triped")
    (build
      (branch (alias Omni) (seed 1)
        (face (alias BotX) (grow 10 (scale .9) (mark :end)))
        (face (alias BotY) (grow 10 (scale .9) (mark :end)))
        (face (alias BotZ) (grow 10 (scale .9) (mark :end)))))
    (shape
      (during 20000 (space :end 0.4))
      (during 20000 (vulcanize))
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Convergence")
    (build
      (branch (alias Omni) (seed 1)
        (face (alias Bot) (grow 2 (scale .9)))
        (face (alias TopY) (grow 10 (scale .9) (mark :end)))
        (face (alias TopX) (grow 10 (scale .9) (mark :end)))
        (face (alias TopZ) (grow 10 (scale .9) (mark :end)))))
    (shape
      (during 18000 (join :end (seed 1)))
      (during 20000 (vulcanize))
      (faces-to-triangles))
    (pretense (surface :frozen)))  (fabric
    (name "Headless Hug")
    (build
      (branch (alias Torque)
        (face (alias LeftFrontBottom) (grow 6 (mark :legs)))
        (face (alias RightFrontBottom) (grow 6 (mark :legs)))
        (face (alias LeftBackTop)
          (grow 2
            (branch (alias Omni) (rotate) (rotate)
              (face (alias TopZ) (mark :chest-1))
              (face (alias BotX) (mark :chest-2))
              (face (alias BotY) (grow 6 (scale .93) (mark :hands))))))
        (face (alias RightBackTop)
          (grow 2
            (branch (alias Omni)
              (face (alias TopY) (mark :chest-1))
              (face (alias BotZ) (mark :chest-2))
              (face (alias BotX) (grow 6 (scale .93) (mark :hands))))))))
    (shape
      (down :legs)
      (during 15000 (space :legs .02) (space :hands .02) (space :chest-1 .4) (space :chest-2 .1))
      (during 80000 (vulcanize))
      (remove-spacers)
      (faces-to-triangles))
    (pretense
      (surface :frozen)))
  (fabric (name "Torque Conundrum")
    (build
      (branch (alias TorqueRight)))
    (shape))
  (fabric (name "Tworque Walker")
    (build
      (branch (alias Torque)
        (face (alias LeftFrontBottom) (grow 1 (branch (alias TorqueLeft))))
        (face (alias LeftBackBottom) (grow 1 (branch (alias TorqueRight))))
        (face (alias RightFrontBottom) (grow 1 (branch (alias TorqueLeft))))
        (face (alias RightBackBottom) (grow 1 (branch (alias TorqueRight) )))
        )
      )
    (shape (faces-to-triangles))
    (pretense
      (muscle 0.5 22000)
      (surface :bouncy))
    )
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
        (face (alias TopRight) (grow 3 (scale .9)))
        (face (alias BottomRight) (grow 3 (scale .9)))
        (face (alias BackLeft) (grow 3 (scale .9)))
        (face (alias FrontLeft) (grow 3 (scale .9)))))
    (shape
      (vulcanize)
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Twisted Infinity")
    (build
      (branch (alias Omni)
        (face (alias TopRight) (grow 12 (scale .92) (mark :ring-a)))
        (face (alias BottomRight) (grow 13 (scale .92) (mark :ring-a)))
        (face (alias BackLeft) (grow 12 (scale .92) (mark :ring-b)))
        (face (alias FrontLeft) (grow 13 (scale .92) (mark :ring-b)))
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
    (name "Pulsating Pavilion")
    (build
      (branch (alias Torque)
        (face (alias LeftFrontBottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias FarBase) (grow 8 (scale 0.93) (mark :a))))))
        (face (alias LeftBackBottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias FarBase) (grow 8 (scale 0.93) (mark :b))))))
        (face (alias RightFrontBottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias FarBase) (grow 8 (scale 0.93) (mark :b))))))
        (face (alias RightBackBottom)
          (grow 1
            (branch (alias Torque) (scale .6) (rotate)
              (face (alias FarBase) (grow 8 (scale 0.93)  (mark :a))))))))
    (shape
      (during 35000 (space :a .4) (space :b .4))
      (during 35000 (vulcanize))
      (faces-to-triangles))
    (pretense
      (surface :frozen)
      (muscle 0.5 8000)
      ))
  (fabric
    (name "Tommy Torque")
    (build
      (branch (alias Omni)
        (face (alias BottomLeft)
          (grow 1
            (branch (alias Torque) (scale .7) (rotate)
              (face (alias FarFront) (grow 1 (mark :sole))))))
        (face (alias BottomRight)
          (grow 1
            (branch (alias Torque) (scale .7)
              (face (alias FarFront) (grow 1 (mark :sole))))))
        (face (alias BackRight)
          (branch (alias Torque)
            (face (alias FarBase)
              (grow 3 (scale .8)))))
        (face (alias BackLeft)
          (branch (alias Torque) (rotate)
            (face (alias FarBase)
              (grow 3 (scale .8)))))))
    (shape
      (down :sole)
      (faces-to-triangles))
    (pretense
      (surface :frozen)
      (muscle 0.3 12000)
      ))
  (fabric (name "Torque Island")
    (build
      (branch (alias Torque)
        (face (alias LeftFrontBottom)
          (branch (alias Torque) (rotate) (rotate)
            (face (alias FarSide) (mark :loose))))
        (face (alias RightFrontBottom)
          (branch (alias Torque) (rotate) (rotate)
            (face (alias FarSide)
              (branch (alias Torque)
                (face (alias FarSide) (mark :loose))))))))
    (shape
      (join :loose)
      (faces-to-triangles))
    (pretense
      (surface :bouncy)
      (muscle 0.3 12000)))
  (fabric
    (name "Torkey")
    (build
      (branch (alias Torque)
        (face (alias LeftFrontTop) (grow 3 (scale .5) (grow 5)))
        (face (alias LeftBackTop) (grow 3 (scale .5) (grow 5)))
        (face (alias LeftFrontBottom)
          (grow 1 (scale 1.2)
            (branch (alias Torque) (group 1) (scale .6) (rotate)
              (face (alias FarBase) (grow 3 (scale 0.7) (mark :paw))))))
        (face (alias LeftBackBottom)
          (grow 1 (scale 1.2)
            (branch (alias Torque) (group 2) (scale .6) (rotate)
              (face (alias FarBase) (grow 3 (scale 0.7) (mark :paw))))))
        (face (alias RightFrontBottom)
          (grow 1
            (branch (alias Torque) (group 1) (scale .6)  (rotate)
              (face (alias FarBase) (grow 2 (scale 0.7) (mark :paw))))))
        (face (alias RightBackBottom)
          (grow 1
            (branch (alias Torque) (group 2) (scale .6) (rotate)
              (face (alias FarBase) (grow 2 (scale 0.7) (mark :paw))))))))
    (shape
      (during 35000 (space :paw .5))
      (during 35000 (vulcanize))
      (faces-to-triangles))
    (pretense
      (surface :sticky)
      (muscle 0.8 10000 1)
      ))
  )