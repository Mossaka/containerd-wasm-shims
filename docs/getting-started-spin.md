# Getting Started

Assuming you have Rust, `containerd`, `docker`, and `docker buildx` installed, which you should assuming you've built, installed, and loaded `runwasi`, all you are missing is `cni`. 

You can install it by running:
```sh
mkdir -p /opt/cni/bin/
curl -SLf https://github.com/containernetworking/plugins/releases/download/v1.1.1/cni-plugins-linux-amd64-v1.1.1.tgz | sudo tar -C /opt/cni/bin -zxv
```

> Note: This is using the release 1.1.1, feel free to update this command to a later release if needed.

After that, run the following:
```sh
mkdir -p /etc/cni/net.d
cat >/etc/cni/net.d/10-mynet.conf <<EOF
{
	"cniVersion": "0.2.0",
	"name": "mynet",
	"type": "bridge",
	"bridge": "cni0",
	"isGateway": true,
	"ipMasq": true,
	"ipam": {
		"type": "host-local",
		"subnet": "10.22.0.0/16",
		"routes": [
			{ "dst": "0.0.0.0/0" }
		]
	}
}
EOF
cat >/etc/cni/net.d/99-loopback.conf <<EOF
{
	"cniVersion": "0.2.0",
	"name": "lo",
	"type": "loopback"
}
EOF
```

> Note: To be able to edit these files like such, I had to first run `sudo su`.

After this, you should be able to run:
1. `make build`,
1. `make install`,
1. `make load`, 
1. in another terminal: `containerd`, and
1. `make run_spink`.

> Note: If there are any issues w/ `make load`, you can always run the image from the ghcr registry. To do so, use: `sudo ctr run --cni --rm --runtime=io.containerd.spin.v1 ghcr.io/library/spinkitchensink:latest testspin`