{
  pkgs ? import <nixpkgs> { },
}:
pkgs.callPackage ./package.nix {
  inherit (pkgs.llvmPackages) stdenv;
  CMAKE_BUILD_TYPE = "Debug";
}
