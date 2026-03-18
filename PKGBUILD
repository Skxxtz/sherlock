# Maintainer: Sebastian Kootz - skxxtz@gmail.com
pkgname="sherlock-launcher-gpui-bin"
pkgver=0.1.1
pkgrel=1
pkgdesc="Application launcher for Wayland."
arch=('x86_64')
url="https://github.com/Skxxtz/sherlock-gpui"
license=('custom:CC-BY-NC-4.0')
depends=('gtk4-layer-shell')
conflicts=('sherlock-launcher-git' 'sherlock-launcher-bin' 'sherlock-launcher-unstable-git')
source=("https://github.com/Skxxtz/sherlock-gpui/releases/download/v${pkgver}/sherlock-v${pkgver}-x86_64.tar.gz")
sha256sums=('e9814f3ea9c55d0b9f6293c511460c57b326d7870313351ecae2ac38ff61641c')

package() {
    install -Dm755 "$srcdir/sherlock" "$pkgdir/usr/bin/sherlock"
    install -Dm644 "$srcdir/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
