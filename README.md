<h1 align="center">
  Glasswing Power
</h1>

<h3 align="center">
    Your tech stack doesn't need so much energy ⚡
</h3>

---

**Glasswing Power** is a metrology agent dedicated to electric [power](https://en.wikipedia.org/wiki/Electric_power) and energy consumption metrics. The goal of the project is to permit to any company or individual to **measure** the power consumption of its tech services and get this data in a convenient form, sending it through any monitoring or data analysis toolchain.

This project is a fork of [Scaphandre](https://github.com/hubblo-org/scaphandre) by [Benoit Petit](https://github.com/bpetit) and the [hubblo-org](https://github.com/hubblo-org) team. We are grateful for their original work and continue to build upon it.

**Warning**: this is still a very early stage project. Any feedback or contribution will be highly appreciated.

## ✨ Features

- measuring power/energy consumed on **bare metal hosts**
- measuring power/energy consumed of **qemu/kvm virtual machines** from the host
- **exposing** power/energy metrics of a virtual machine, to allow **manipulating those metrics in the VM** as if it was a bare metal machine (relies on hypervisor features)
- exposing metrics as a **[prometheus](https://prometheus.io) (HTTP) exporter**
- sending metrics in push mode to a **[prometheus](https://prometheus.io) [Push Gateway](https://github.com/prometheus/pushgateway)**
- sending metrics to **[riemann](http://riemann.io/)**
- sending metrics to **[Warp10](http://warp10.io/)**
- works on **[kubernetes](https://kubernetes.io/)**
- storing power consumption metrics in a **JSON** file
- showing basic power consumption metrics **in the terminal**
- operating systems supported so far : **Gnu/Linux**, **Windows 10, 11 and Server 2016/2019/2022**
- packages available for **RHEL 8 and 9, Debian 11 and 12 and Windows**, also **NixOS** (community support)

## 📄 How to ... ?

You'll find everything you may want to know about glasswing-power in the [documentation](https://hubblo-org.github.io/scaphandre-documentation), like:

- 🏁 [Getting started](https://hubblo-org.github.io/scaphandre-documentation/tutorials/getting_started.html)
- 💻 [Installation & compilation on GNU/Linux](https://hubblo-org.github.io/scaphandre-documentation/tutorials/installation-linux.html) or [on Windows](https://hubblo-org.github.io/scaphandre-documentation/tutorials/installation-windows.html)
- 👁️ [Give a virtual machine access to its power consumption metrics, and break the opacity of being on the computer of someone else](https://hubblo-org.github.io/scaphandre-documentation/how-to_guides/propagate-metrics-hypervisor-to-vm_qemu-kvm.html)
- 🎉 [Contributing guide](https://hubblo-org.github.io/scaphandre-documentation/contributing.html)
- [And much more](https://hubblo-org.github.io/scaphandre-documentation)

If you are only interested in the code documentation [here it is](https://docs.rs/glasswing-power).

## ⚖️  Footprint

In opposition to its name, glasswing-power aims to be as light and clean as possible. One of the main focus areas of the project is to come as close as possible to a 0 overhead, both about resources consumption and power consumption.

## 🙏 Acknowledgments

This project is a fork of [Scaphandre](https://github.com/hubblo-org/scaphandre) by [Benoit Petit](https://github.com/bpetit) and the [hubblo-org](https://github.com/hubblo-org) team. We are grateful for their original work and continue to build upon it.

## ⚖️ License

MIT License - see [LICENSE](LICENSE) for details.
