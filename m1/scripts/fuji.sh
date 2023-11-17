VM_NAME="movement"
VM_ID="b6z34iYog6Qm8uUWmssHYWDhShhufDBtLo75K9nHwU69GTs8o"
SUBNET_ID="K4GygGTpKkNzzjiLfZVsmQduGqSFztJx4nk52CvA1afcFAhsH"

cargo build -p subnet
cp ./target/debug/subnet ~/.avalanchego/plugins/$VM_ID
./avalanchego --http-host=0.0.0.0 --network-id="fuji" --track-subnets "$SUBNET_ID"