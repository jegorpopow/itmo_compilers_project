{
  lib,
  stdenv,
  cmake,
  gtest,
  gflags,
  fmt_11,
  CMAKE_BUILD_TYPE ? "Release",
}:
let
  compiler = stdenv.cc.meta.name;
  pname = "vm";
  version = "${CMAKE_BUILD_TYPE}-${compiler}";
  mainProgram = "${pname}-${CMAKE_BUILD_TYPE}";
in
stdenv.mkDerivation {
  inherit pname version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./CMakeLists.txt
      ./include
      ./src
      ./tests
    ];
  };

  nativeBuildInputs = [ cmake ];
  cmakeFlags = [ "-DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}" ];
  buildInputs = [
    gtest
    gflags
    fmt_11
  ];

  doCheck = true;

  postInstall = ''
    mv $out/bin/{${pname},${mainProgram}}
  '';

  meta = {
    inherit mainProgram;
    branch = CMAKE_BUILD_TYPE;
    platforms = lib.platforms.linux;
  };
}
