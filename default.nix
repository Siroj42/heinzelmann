{ lib, rustPlatform }:

rustPlatform.buildRustPackage rec {
	pname = "heinzelmann";
	version = "0.1.0";

	src = ./.;
	cargoHash = "sha256-DuV5cPjRPkinHB9BrjpQpVE/tqPGz4y9H26YpctU9R4=";

	meta = with lib; {
		description = "A program for automating MQTT using the Steel dialect of Scheme";
		license = licenses.agpl3Plus;
	};
}
