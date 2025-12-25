# Maintainer: Alessio Deiana <adeiana@gmail.com>
pkgname=iostat
pkgver=0.1.0
pkgrel=1
pkgdesc="I/O statistics reporter written in Rust"
arch=('x86_64')
url="https://github.com/osso/iostat"
license=('MIT')
depends=('gcc-libs')
makedepends=('cargo')
provides=('iostat')
conflicts=('sysstat')
source=()

build() {
    cd "$startdir"
    cargo build --release --locked
}

package() {
    cd "$startdir"
    install -Dm755 "target/release/iostat" "$pkgdir/usr/bin/iostat"
}
