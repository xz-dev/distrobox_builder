use std::collections::HashMap;

use crate::container_tree::builder::build_container_trees;
use crate::container_tree::distrobox_assemble_tree::{trees_to_distrobox_assemble, ContainerNode};
use crate::distrobox::parser::assemble::ContainerAssembleData;
use crate::oci::command_helper::build_image_from_dockerfile_simple;
use crate::oci::image_builder::build_image;
use crate::utils::command_helper::run_command;

fn build_image_by_tree(container_runner: &str, tree: &mut ContainerNode) {
    fn tree_to_image_map(container_runner: &str, tree: &mut ContainerNode, node_level: usize) {
        let image = &tree.container_assemble_data.image.clone();
        if image.starts_with("dockerfile://") {
            println!("Build dockerfile: {}", &image);
            let image_name = format!("distrobox-dockerfile_{}", &tree.container_name);
            let dockerfile_path = &image[12..];
            let output = build_image_from_dockerfile_simple(
                container_runner,
                &image_name,
                &dockerfile_path,
                ".",
            )
            .unwrap();
            if output.status.is_some_and(|status| status == 0) {
                tree.container_assemble_data.image = image_name;
            }
        }
        let image = &tree.container_assemble_data.image.clone();
        println!("Build image: {}", &image);
        let empty_vec = vec![];
        let packages = tree
            .container_assemble_data
            .packages
            .as_ref()
            .unwrap_or(&empty_vec);
        println!("Packages: {:?}", &packages);
        let new_image = format!("distrobox-{}-{}", node_level, &image);
        println!(
            "Build container name: {} to {}",
            &tree.container_name, &new_image
        );
        if tree.container_assemble_data.pre_build_image.is_some() {
            let pre_build_image = tree
                .container_assemble_data
                .pre_build_image
                .clone()
                .unwrap();
            println!("Pre build image: {}", &pre_build_image);
            let command_name = &pre_build_image.split_whitespace().next().unwrap();
            let args = &pre_build_image
                .split_whitespace()
                .skip(1)
                .collect::<Vec<&str>>();
            run_command(command_name, args, true).unwrap();
        }
        build_image(
            container_runner,
            &new_image,
            &image,
            &tree.container_assemble_data.package_manager,
            packages,
        )
        .unwrap();
        tree.container_assemble_data.image = new_image.clone();
        for child in &mut tree.children {
            tree_to_image_map(container_runner, child, node_level + 1);
        }
    }
    tree_to_image_map(container_runner, tree, 0);
}

pub fn build_distrobox_assemble_data(
    container_runner: &str,
    data: &HashMap<String, ContainerAssembleData>,
) -> HashMap<String, ContainerAssembleData> {
    let mut trees = build_container_trees(data);

    for tree in &mut trees {
        build_image_by_tree(container_runner, tree);
    }

    let mut new_data = trees_to_distrobox_assemble(&trees);
    for (key, value) in new_data.iter_mut() {
        value.pull = Some(false);
        value.packages = data[key].packages.clone();
    }
    new_data
}
