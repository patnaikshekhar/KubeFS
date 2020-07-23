# KubeFS

A FUSE filesystem for Kubernetes. Lets you mount your kubernetes cluster as a filesystem on a linux machine. Once mounted you can explore the filesystem using standard *nix commands such as ls, cd, mkdir, rmdir, cat, etc.

## Usage

```bash
kubefs <mountpath>
```

## Features
- Lists namespaces, pods, deployments, configmaps, etc using **ls**
- Create namespaces with **mkdir**
- View manifests by navigating to path and using **cat**
- Delete namespace with **rmdir**
- Update manifests by using **vim** or **nano**

## Demo
[![Demo](https://img.youtube.com/vi/cPuOBHO8Ajg/0.jpg)](https://youtu.be/cPuOBHO8Ajg)