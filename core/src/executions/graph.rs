use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    panic::{RefUnwindSafe, UnwindSafe},
    sync::Arc,
    thread::scope,
};

use anyhow::{anyhow, bail, Context};
use blaze_common::{
    dependency::Dependency,
    error::{Error, Result},
    executor::ExecutorReference,
    parallelism::Parallelism,
    project::Project,
    workspace::Workspace,
};

use crate::{
    system::parallel_executor::ParallelRunner,
    workspace::{
        configurations::DeserializationContext,
        project_handle::{ProjectHandle, ProjectOptions},
        selection::{Selection, SelectionContext, SelectorSource},
    },
};

use super::execution::TargetExecution;

type DependencyGraph = HashMap<String, DependencyGraphNode>;

/// Provides our main graph execution logic.
/// It handles dependencies resolution on instanciation as well as parallel execution model.
/// The execution routine for each target is user-provided.
#[derive(Debug)]
pub struct ExecutionGraph {
    dependency_graph: DependencyGraph,
}

/// Data needed when instanciating an [`ExecutionGraph`].
pub struct ExecutionGraphOptions<'a> {
    pub workspace: &'a Workspace,
    pub deserialization_context: DeserializationContext<'a>,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone)]
struct DependencyGraphNode {
    root: bool,
    target_execution: Arc<TargetExecution>,
    dependencies: BTreeMap<String, Arc<DependencyAccessor>>,
}

impl<T> ExecutedNode<T> {
    fn new(
        root: bool,
        execution: TargetExecution,
        dependencies: HashSet<String>,
        return_value: Option<Result<T>>,
    ) -> Self {
        Self {
            root,
            execution,
            dependencies: HashSet::from_iter(
                dependencies.into_iter().map(|double| double.to_string()),
            ),
            result: return_value,
        }
    }
}

struct DependenciesResolution {
    selection: Option<Selection>,
    target: String,
    ancestors: Vec<(String, DependencyAccessor)>,
    ancestors_set: HashSet<String>,
    depth: usize,
}

#[derive(Debug, Clone)]
pub struct DependencyAccessor {
    src_project: Arc<Project>,
    src_target: String,
    dependency_index: usize,
}

impl AsRef<Dependency> for DependencyAccessor {
    fn as_ref(&self) -> &Dependency {
        &self.src_project.targets()[&self.src_target].dependencies()[self.dependency_index]
    }
}

/// Represents the targets graph state after execution.
#[derive(Debug)]
pub struct ExecutedGraph<T> {
    executions: BTreeMap<String, ExecutedNode<T>>,
}

impl<T> ExecutedGraph<T> {
    pub fn empty() -> Self {
        Self {
            executions: BTreeMap::default(),
        }
    }

    pub fn execution(&self) -> &BTreeMap<String, ExecutedNode<T>> {
        &self.executions
    }

    pub fn root_executions(&self) -> BTreeMap<&String, &ExecutedNode<T>> {
        self.executions
            .iter()
            .filter(|(_, execution)| execution.root)
            .collect()
    }

    pub fn map_inner<O, F: FnMut(T) -> O>(self, mut f: F) -> ExecutedGraph<O> {
        ExecutedGraph {
            executions: self
                .executions
                .into_iter()
                .map(|(double, execution_result)| {
                    (
                        double,
                        ExecutedNode {
                            root: execution_result.root,
                            dependencies: execution_result.dependencies,
                            execution: execution_result.execution,
                            result: execution_result.result.map(|result| result.map(&mut f)),
                        },
                    )
                })
                .collect(),
        }
    }
}

/// A single target after execution.
#[derive(Debug)]
pub struct ExecutedNode<T> {
    pub root: bool,
    pub execution: TargetExecution,
    pub dependencies: HashSet<String>,
    pub result: Option<Result<T>>,
}

pub struct InternalChildExecutionResult<T> {
    pub execution: Arc<TargetExecution>,
    pub dependency: Arc<DependencyAccessor>,
    pub result: Option<Arc<Result<T>>>,
}

/// A single child target after execution.
pub struct ChildExecutionResult<'a, 'b, T> {
    pub execution: &'a TargetExecution,
    pub dependency: &'a DependencyAccessor,
    pub result: Option<&'b Result<T>>,
}

impl ExecutionGraph {
    /// Create a new execution graph using the provided options and context.
    /// All dependencies will be resolved recursively.
    pub fn try_new(
        selection: &Selection,
        target: &str,
        options: ExecutionGraphOptions<'_>,
    ) -> Result<Self> {
        let mut dependency_graph = HashMap::<String, DependencyGraphNode>::new();

        let mut projects = HashMap::<String, Arc<Project>>::new();
        let mut resolutions = VecDeque::from([DependenciesResolution {
            selection: Some(selection.clone()),
            target: target.to_owned(),
            ancestors: vec![],
            ancestors_set: HashSet::new(),
            depth: 0,
        }]);

        while let Some(DependenciesResolution {
            selection,
            target,
            ancestors,
            ancestors_set,
            depth,
        }) = resolutions.pop_front()
        {
            let project_names = match selection {
                Some(selection) => {
                    let refs = selection
                        .select(SelectionContext {
                            workspace: options.workspace,
                        })
                        .context("could not select projects")?;

                    for (name, project_ref) in &refs {
                        if !projects.contains_key(*name) {
                            projects.insert(
                                (*name).to_owned(),
                                Arc::new(
                                    ProjectHandle::from_root(
                                        options.workspace.root().join(project_ref.path()),
                                        ProjectOptions {
                                            name,
                                            deserialization_context: options
                                                .deserialization_context,
                                        },
                                    )
                                    .with_context(|| {
                                        format!(
                                            "error while reading \"{name}\" project configuration"
                                        )
                                    })?
                                    .unwrap_inner(),
                                ),
                            );
                        }
                    }
                    refs.keys().map(|name| name.as_str()).collect::<Vec<_>>()
                }
                None => vec![ancestors.last().unwrap().1.src_project.name()],
            };

            for project_name in project_names {
                let project = projects[project_name].clone();
                if let Some(target_execution) = TargetExecution::try_new(project.clone(), &target) {
                    let double = target_execution.get_double();

                    if ancestors_set.contains(&double) {
                        let circular_ancestor_position = ancestors
                            .iter()
                            .position(|(ancestor, _)| &double == ancestor)
                            .unwrap();
                        let mut chain = ancestors[circular_ancestor_position..]
                            .iter()
                            .map(|(double, _)| double.as_str())
                            .collect::<Vec<_>>();
                        chain.push(double.as_str());
                        bail!("circular dependency detected ({})", chain.join(" <=> "))
                    }

                    if let Some((ancestor_double, dependency_accessor)) = ancestors.last().cloned()
                    {
                        dependency_graph
                            .get_mut(&ancestor_double)
                            .unwrap()
                            .dependencies
                            .insert(double.to_owned(), Arc::new(dependency_accessor));
                    }

                    if dependency_graph.contains_key(&double) {
                        continue;
                    }

                    let target_execution = Arc::new(target_execution);

                    let _ = dependency_graph.insert(
                        double.clone(),
                        DependencyGraphNode {
                            root: depth == 0,
                            target_execution: target_execution.clone(),
                            dependencies: BTreeMap::new(),
                        },
                    );

                    if options.max_depth.is_some_and(|max| depth >= max) {
                        continue;
                    }

                    for (i, dependency) in target_execution
                        .get_target()
                        .dependencies()
                        .iter()
                        .enumerate()
                    {
                        let mut next_ancestors = ancestors.clone();
                        next_ancestors.push((
                            double.to_owned(),
                            DependencyAccessor {
                                src_target: target_execution.get_target_name().to_owned(),
                                src_project: project.clone(),
                                dependency_index: i,
                            },
                        ));
                        let mut next_ancestors_set = ancestors_set.clone();
                        next_ancestors_set.insert(double.to_owned());

                        resolutions.push_back(DependenciesResolution {
                            selection: dependency.projects().cloned().map(|selector| {
                                Selection::from_source(SelectorSource::Provided(selector))
                            }),
                            target: dependency.target().to_owned(),
                            ancestors: next_ancestors,
                            ancestors_set: next_ancestors_set,
                            depth: depth + 1,
                        })
                    }
                }
            }
        }

        Ok(Self { dependency_graph })
    }

    /// Get a list of all executor URLs required to execute this graph.
    pub fn get_executor_references(&self) -> HashSet<ExecutorReference> {
        self.dependency_graph
            .values()
            .filter_map(|node| node.target_execution.get_target().executor())
            .cloned()
            .collect()
    }

    /// Get a list of all execution doubles in this graph.
    pub fn targets(&self) -> Vec<&str> {
        self.dependency_graph.keys().map(String::as_str).collect()
    }

    /// Execute all targets using this graph with the specified parallelism level and execution routine.
    pub fn execute<
        T: Send + Sync + UnwindSafe + RefUnwindSafe,
        F: Fn(&TargetExecution, &[ChildExecutionResult<T>]) -> Result<T> + Clone + UnwindSafe + Send,
    >(
        self,
        parallelism: Parallelism,
        execution_routine: F,
    ) -> Result<ExecutedGraph<T>> {
        scope(|scope| {
            let mut parallel_executor = ParallelRunner::new(scope, parallelism)?;

            let mut pending = self.dependency_graph.keys().collect::<HashSet<&String>>();

            let mut results =
                HashMap::<String, Arc<Result<T>>>::with_capacity(self.dependency_graph.len());

            let mut canceled = HashSet::<&String>::with_capacity(self.dependency_graph.len() - 1);

            let inverted_dependencies = self.create_inverted_dependency_graph();

            loop {
                let mut next_doubles = pending
                    .iter()
                    .filter(|double| {
                        self.dependency_graph[**double].dependencies.iter().all(
                            |(dependency_double, source_accessor)| {
                                if AsRef::<Dependency>::as_ref(source_accessor.as_ref()).optional()
                                {
                                    return canceled.contains(dependency_double)
                                        || results.contains_key(dependency_double);
                                }

                                results
                                    .get(dependency_double)
                                    .filter(|result| result.is_ok())
                                    .is_some()
                            },
                        )
                    })
                    .map(|double| {
                        self.dependency_graph
                            .get_key_value(*double)
                            .unwrap()
                            .0
                            .to_owned()
                    })
                    .collect::<Vec<_>>();

                parallel_executor.push_available(|| {
                    let double = next_doubles.pop()?;
                    let node = &self.dependency_graph[&double];

                    let internal_child_executions = node
                        .dependencies
                        .iter()
                        .map(|(dependency_double, dependency_accessor)| {
                            InternalChildExecutionResult {
                                execution: self.dependency_graph[dependency_double]
                                    .target_execution
                                    .clone(),
                                dependency: dependency_accessor.clone(),
                                result: results.get(dependency_double.as_str()).cloned(),
                            }
                        })
                        .collect::<Vec<_>>();

                    let target_execution_1 = node.target_execution.clone();
                    let execution_routine_clone = execution_routine.clone();

                    pending.remove(&double);

                    Some(move || {
                        let result = execution_routine_clone(
                            target_execution_1.as_ref(),
                            internal_child_executions
                                .iter()
                                .map(|child| ChildExecutionResult {
                                    execution: child.execution.as_ref(),
                                    dependency: child.dependency.as_ref(),
                                    result: child.result.as_ref().map(|arc| arc.as_ref()),
                                })
                                .collect::<Vec<_>>()
                                .as_slice(),
                        );
                        (double.to_owned(), result)
                    })
                });

                if !parallel_executor.is_running() && pending.is_empty() {
                    break;
                }

                for (done_double, result) in parallel_executor.drain()? {
                    let is_ok = result.is_ok();

                    results.insert(done_double.to_owned(), Arc::new(result));

                    if is_ok {
                        continue;
                    }

                    let mut to_cancel = HashSet::<&String>::with_capacity(pending.len());

                    let mut next_children = HashSet::<&String>::from_iter([&done_double]);

                    loop {
                        if next_children.is_empty() {
                            break;
                        }

                        let next_parents = next_children
                            .iter()
                            .flat_map(|child_double| inverted_dependencies[*child_double].iter())
                            .collect::<HashMap<_, _>>();

                        next_children.clear();

                        for (parent_double, dependency_accessor) in next_parents {
                            if AsRef::<Dependency>::as_ref(dependency_accessor.as_ref()).optional()
                            {
                                continue;
                            }

                            to_cancel.insert(parent_double);
                            next_children.insert(parent_double);
                        }
                    }

                    canceled.extend(to_cancel);
                    pending.retain(|double| !canceled.contains(double));
                }
            }

            Ok(ExecutedGraph {
                executions: self
                    .dependency_graph
                    .into_iter()
                    .map(|(double, node)| {
                        let execution_result = ExecutedNode::new(
                            node.root,
                            Arc::try_unwrap(node.target_execution).map_err(arc_error)?,
                            node.dependencies.keys().cloned().collect(),
                            results
                                .remove(double.as_str())
                                .map(Arc::try_unwrap)
                                .transpose()
                                .map_err(arc_error)?,
                        );
                        Ok((double, execution_result))
                    })
                    .collect::<Result<_>>()?,
            })
        })
    }

    /// Same as the [`execute`] method from the [`ExecutionGraph`], but without actually executing targets.
    /// All targets will be returned without any result (which means a [`None`] value).
    pub fn ignore_all<T>(self) -> Result<ExecutedGraph<T>> {
        Ok(ExecutedGraph {
            executions: self
                .dependency_graph
                .into_iter()
                .map(|(double, node)| {
                    Ok((
                        double,
                        ExecutedNode::new(
                            node.root,
                            Arc::try_unwrap(node.target_execution).map_err(arc_error)?,
                            node.dependencies.keys().cloned().collect(),
                            None,
                        ),
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }

    /// Create an inverted dependencies graph (from dependency to parents).
    /// Keys are dependencies doubles and values are maps where keys are parent target names and values are dependency configuration accessors.
    fn create_inverted_dependency_graph(
        &self,
    ) -> HashMap<&String, HashMap<&String, Arc<DependencyAccessor>>> {
        let mut inverted_dependencies = self
            .dependency_graph
            .keys()
            .map(|double| (double, HashMap::new()))
            .collect::<HashMap<_, _>>();

        for (parent_double, parent_node) in &self.dependency_graph {
            for (dependency_double, dependency_accessor) in &parent_node.dependencies {
                inverted_dependencies
                    .get_mut(dependency_double)
                    .unwrap()
                    .insert(parent_double, dependency_accessor.clone());
            }
        }

        inverted_dependencies
    }
}

impl<T> ExecutedGraph<T> {
    pub fn fmt<O: std::io::Write, F: Fn(&ExecutedNode<T>) -> String>(
        &self,
        output: &mut O,
        formatter: F,
    ) -> Result<()> {
        for node in self.root_executions().values() {
            let mut nodes = VecDeque::from([(0_usize, *node)]);
            while let Some(next) = nodes.pop_front() {
                let mut arrow = String::with_capacity(100);

                for i in 0..next.0 {
                    if i < next.0 - 1 {
                        arrow.push_str("│   ");
                    } else {
                        arrow.push_str("├── ");
                    }
                }

                output.write_all(
                    &[
                        arrow.as_bytes(),
                        formatter(next.1).as_bytes(),
                        "\n".as_bytes(),
                    ]
                    .concat(),
                )?;

                for child in next
                    .1
                    .dependencies
                    .iter()
                    .map(|dep_name| (next.0 + 1, &self.executions[dep_name]))
                {
                    nodes.push_front(child);
                }
            }
        }

        Ok(())
    }
}

/// Optimize the whole graph so that we remove redondant relations between nodes.
/// based on the work https://gist.github.com/matejker/6d9305e23a168ed66d3260eb261bb98b
#[allow(unused)]
#[deprecated]
fn optimize_dependency_graph(dependency_graph: &DependencyGraph) -> DependencyGraph {
    let mut dependencies_to_remove = dependency_graph
        .keys()
        .map(|target| (target, Vec::<&String>::new()))
        .collect::<BTreeMap<_, _>>();

    for (target, node) in dependency_graph {
        let combinations = node
            .dependencies
            .keys()
            .flat_map(|dependency| {
                node.dependencies
                    .keys()
                    .map(move |other_dependency| (dependency, other_dependency))
            })
            .filter(|(dependency, other_dependency)| dependency != other_dependency);

        for (dependency, other_dependency) in combinations {
            if dependency_graph[other_dependency]
                .dependencies
                .contains_key(dependency)
            {
                dependencies_to_remove
                    .get_mut(target)
                    .unwrap()
                    .push(dependency);
            }
        }
    }

    let mut optimized_graph = dependency_graph.clone();

    for (target, duplicates) in dependencies_to_remove {
        optimized_graph
            .get_mut(target)
            .unwrap()
            .dependencies
            .retain(|dependency, _| !duplicates.contains(&dependency));
    }

    optimized_graph
}

fn arc_error<T>(_: Arc<T>) -> Error {
    anyhow!("arc unwrap error.")
}
