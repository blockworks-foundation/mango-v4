// https://github.com/Vincent-Pang/builder-pattern
export function Builder(typeOrTemplate, templateOrOverride, override) {
    let type;
    let template;
    let overrideValues;
    if (typeOrTemplate instanceof Function) {
        type = typeOrTemplate;
        template = templateOrOverride;
        overrideValues = override;
    }
    else {
        template = typeOrTemplate;
        overrideValues = templateOrOverride;
    }
    const built = template
        ? Object.assign({}, template)
        : {};
    const builder = new Proxy({}, {
        get(target, prop) {
            if ('build' === prop) {
                if (overrideValues) {
                    Object.assign(built, overrideValues);
                }
                if (type) {
                    // A class name (identified by the constructor) was passed. Instantiate it with props.
                    const obj = new type();
                    return () => Object.assign(obj, { ...built });
                }
                else {
                    // No type information - just return the object.
                    return () => built;
                }
            }
            return (...args) => {
                // If no arguments passed return current value.
                if (0 === args.length) {
                    return built[prop.toString()];
                }
                built[prop.toString()] = args[0];
                return builder;
            };
        },
    });
    return builder;
}
