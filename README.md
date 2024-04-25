# Tensegrity Lab

The tensegrity lab project is an exploration of the design space of structures based on purely tension and compression using the [Rust](https://www.rust-lang.org/) programming language and [WGPU](https://wgpu.rs/) for graphics.

Originally inspired by the work of [Kenneth Snelson](http://kennethsnelson.net/), this project explores what more is possible based on these design principles.

The ultimate goal is to enable the design and construction of elaborate real-world physical tensegrity structures which realistically cannot be designed by hand. Examples and construction stories can be found on [pretenst.com](https://pretenst.com)

[image]

The foundation is a simple and efficient model and [physics](docs/physics.md) simulation where all elements operate according to Hooke's law and time progresses in discrete steps not unlike a cellular automaton.

The design of elaborate tensegrity structures is made possible through a domain specific language named [tenscript](docs/tenscript.md) and a model based on the notion of bricks which are tensegrity modules connected together by unifying triangular faces.

[image]

## Contact us

If you are interested, we'd love to hear from you, so drop us a line at **pretenst@gmail.com*4*. 
