#!/bin/bash

git config --global user.email "noreply@movementlabs.xyz"
git config --global user.name "CI Bot"

# Clone the Movement Framework Mirror repository
git clone https://github.com/movemntdev/movement-framework-mirror.git

# Copy the aptos-move directory to the Movement Framework Mirror repository
cp -r vm/aptos-vm/aptos-move movement-framework-mirror/

# Change directory to the Movement Framework Mirror repository
cd movement-framework-mirror/

# Add all changes to Git
git add -A

# Commit the changes
git commit -m "Mirror aptos-move directory from current repo"

# Push the changes with force
git push --force
