# Maintainer: Alexander Chaplin Braz <contact@alexchaplinbraz.com>
pkgname='ablavema-bin'
_pkgname='ablavema'
pkgver=REPLACE_RELEASE_VERSION
pkgtarget='x86_64-unknown-linux-gnu'
pkgrel=1
pkgdesc='A Blender launcher and version manager'
arch=('x86_64')
url='https://github.com/AlexChaplinBraz/Ablavema'
license=('MIT')
depends=('gtk3' 'glib2' 'gcc-libs' 'fontconfig' 'freetype2' 'glibc' 'xz' 'bzip2' 'libx11')
conflicts=('ablavema' 'ablavema-git')
source_x86_64=("$url/releases/download/$pkgver/$_pkgname-$pkgver-$pkgtarget.tar.gz")
sha256sums_x86_64=('REPLACE_SHA256SUMS_X86_64')

package() {
	install -Dm644 "$srcdir/$_pkgname-$pkgver-$pkgtarget/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
	install -Dm755 "$srcdir/$_pkgname-$pkgver-$pkgtarget/$_pkgname" "$pkgdir/usr/bin/$_pkgname"
}

