{ lib, rustPlatform }:

rustPlatform.buildRustPackage rec {
  pname = "heinzelmann";
  version = "0.1.0";

  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes."steel-core-0.5.0" = "sha256-qDH7QWWfRlT/RtlaTx+r+mxaTzQFX8HIenslv2OanS8=";
  };

  meta = with lib; {
    description = "A program for automating MQTT using the Steel dialect of Scheme";
    license = licenses.agpl3Plus;
  };
}
