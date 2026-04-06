# Maintainer: ry2x

pkgname=crop-hypr
pkgver=0.4.1
pkgrel=1
pkgdesc="A fast, Hyprland-native screenshot tool written in Rust"
arch=('x86_64')
url="https://github.com/ry2x/crop-hypr"
license=('MIT')
depends=('slurp' 'wl-clipboard' 'hyprland' 'libnotify' 'pipewire')
makedepends=('rust' 'cargo' 'clang' 'pkgconf')
source=()
sha256sums=()

build() {
    cd "$startdir"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    # GCC LTO (-flto=auto) produces GCC IR objects incompatible with Rust's lld.
    # -ffat-lto-objects includes regular machine code alongside LTO IR so lld
    # can resolve symbols from C dependencies (e.g. libspa).
    export CFLAGS+=" -ffat-lto-objects"
    export CXXFLAGS+=" -ffat-lto-objects"
    cargo build --frozen --release
}

check() {
    cd "$startdir"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --frozen
}

package() {
    cd "$startdir"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
