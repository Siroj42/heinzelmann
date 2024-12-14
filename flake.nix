{
  description = "MQTT Home Automation software with Scheme scripting";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let 
        pkgs = nixpkgs.legacyPackages.${system}.extend rust-overlay.overlays.default; 
        buildInputs = with pkgs; [ rust-bin.nightly.latest.default ];
      in
      {
        packages = rec {
          default = heinzelmann;
          heinzelmann = pkgs.callPackage ./default.nix {};
          container = pkgs.dockerTools.buildImage {
            name = "heinzelmann";
            config = { 
              Cmd = [ "${heinzelmann}/bin/heinzelmann" ]; 
              ExposedPorts = { 
                "7888/tcp" = {}; # NRepl port
              };
            };
          };
        };
        apps = rec {
          default = heinzelmann;
          heinzelmann = flake-utils.lib.mkApp { drv = self.packages.${system}.heinzelmann; };
        };
        devShells = rec {
          default = heinzelmann;
          heinzelmann = pkgs.mkShell {
            inherit buildInputs;
          };
        };
      }
    );
}
