{ lib, rustPlatform }:

rustPlatform.buildRustPackage rec {
	pname = "heinzelmann";
	version = "0.1.0";

	src = ./.;
	cargoHash = "sha256-Lznllt1W1+HyJ6YrHgyXxthXuSRRIRYS2dv7kfX6IzA=";

	meta = with lib; {
		description = "A program for automating MQTT using the Steel dialect of Scheme";
		license = licenses.agpl3Plus;
	};
}
