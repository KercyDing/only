use smol_str::SmolStr;

use crate::TaskAst;

pub(crate) fn resolve_dependency_names(tasks: &mut [TaskAst]) {
    let global_tasks = tasks
        .iter()
        .filter(|task| task.namespace.is_none())
        .map(|task| task.name.clone())
        .collect::<std::collections::HashSet<_>>();
    let namespace_tasks = tasks.iter().fold(
        std::collections::HashMap::<SmolStr, std::collections::HashSet<SmolStr>>::new(),
        |mut map, task| {
            if let Some(namespace) = &task.namespace {
                map.entry(namespace.clone())
                    .or_default()
                    .insert(task.name.clone());
            }
            map
        },
    );

    for task in tasks {
        for dependency in &mut task.dependencies {
            if dependency.name.contains('.') {
                continue;
            }

            if let Some(namespace) = &task.namespace
                && namespace_tasks
                    .get(namespace)
                    .is_some_and(|tasks| tasks.contains(&dependency.name))
            {
                dependency.name = SmolStr::from(format!("{namespace}.{}", dependency.name));
                continue;
            }

            if global_tasks.contains(&dependency.name) {
                continue;
            }
        }
    }
}
